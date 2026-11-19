use url::Url;
use webkit6::prelude::*;

pub fn video_id(input: &str) -> Result<String, &'static str> {
    let url = Url::parse(input.trim()).map_err(|_| "Enter a YouTube video URL")?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Use an HTTP or HTTPS YouTube URL");
    }
    let id = match url.host_str() {
        Some("youtube.com" | "www.youtube.com" | "m.youtube.com") => {
            let mut parts = url.path_segments().ok_or("Invalid YouTube URL")?;
            match (parts.next(), parts.next(), parts.next()) {
                (Some("watch"), None, None) => url
                    .query_pairs()
                    .find(|(key, _)| key == "v")
                    .map(|(_, value)| value.into_owned())
                    .ok_or("YouTube watch URL has no video ID")?,
                (Some("live"), Some(id), None) | (Some("embed"), Some(id), None) => {
                    id.to_string()
                }
                _ => return Err("Use a YouTube watch or live URL"),
            }
        }
        Some("youtu.be") => {
            let mut parts = url.path_segments().ok_or("Invalid YouTube URL")?;
            match (parts.next(), parts.next()) {
                (Some(id), None) => id.to_string(),
                _ => return Err("Use a youtu.be video URL"),
            }
        }
        _ => return Err("Use a YouTube URL"),
    };
    if id.len() != 11 || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') {
        return Err("Invalid YouTube video ID");
    }
    Ok(id)
}

pub fn canonical_url(input: &str) -> Result<String, &'static str> {
    Ok(format!("https://www.youtube.com/watch?v={}", video_id(input)?))
}

pub fn web_view(input: &str, muted: bool, looping: bool) -> Result<webkit6::WebView, &'static str> {
    let id = video_id(input)?;
    let view = webkit6::WebView::new();
    if let Some(settings) = webkit6::prelude::WebViewExt::settings(&view) {
        settings.set_media_playback_requires_user_gesture(false);
    }
    view.connect_decide_policy(|_, decision, kind| {
        if kind != webkit6::PolicyDecisionType::NavigationAction {
            return false;
        }
        // The default website policy allows autoplay only without sound.
        decision.use_with_policies(
            &webkit6::WebsitePolicies::builder()
                .autoplay(webkit6::AutoplayPolicy::Allow)
                .build(),
        );
        true
    });
    view.set_is_muted(muted);
    view.set_can_focus(false);
    view.load_html(&player_html(&id, muted, looping), Some(&format!("https://{}/", crate::APP_ID)));
    Ok(view)
}

fn player_html(id: &str, muted: bool, looping: bool) -> String {
    const HTML: &str = r#"<!doctype html><html><head><meta name="referrer" content="strict-origin-when-cross-origin"><meta name="viewport" content="width=device-width,initial-scale=1"><style>html,body,#player,#player iframe{margin:0;width:100%;height:100%;border:0;background:#000;overflow:hidden}#error{display:none;place-items:center;position:fixed;inset:0;background:#111;color:#fff;font:20px sans-serif;text-align:center;padding:2rem}</style></head><body><div id="player"></div><div id="error" role="alert"></div><script src="https://www.youtube.com/iframe_api"></script><script>
let ready=false;
const timer=setTimeout(()=>{if(!ready)fail('YouTube player did not load. Check the network connection.')},30000);
function fail(message){document.getElementById('player').style.display='none';const error=document.getElementById('error');error.textContent=message;error.style.display='grid'}
function onYouTubeIframeAPIReady(){new YT.Player('player',{videoId:'VIDEO_ID',width:'100%',height:'100%',playerVars:{autoplay:1,controls:0,playsinline:1,loop:LOOP_FLAG,playlist:LOOP_FLAG?'VIDEO_ID':undefined,origin:'https://APP_ID'},events:{onReady:function(e){ready=true;clearTimeout(timer);if(MUTED_FLAG)e.target.mute();else e.target.unMute();e.target.playVideo()},onError:function(e){const errors={2:'Invalid YouTube video ID.',5:'This video cannot play in the HTML5 player.',100:'This video is unavailable.',101:'The owner does not allow embedding this video.',150:'The owner does not allow embedding this video.',153:'YouTube could not identify this player.'};fail(errors[e.data]||'YouTube playback error '+e.data)},onAutoplayBlocked:function(){fail('Autoplay was blocked. Try the mute option in Vesper settings.')}}})}
</script></body></html>"#;
    HTML.replace("VIDEO_ID", id)
        .replace("APP_ID", crate::APP_ID)
        .replace("MUTED_FLAG", if muted { "true" } else { "false" })
        .replace("LOOP_FLAG", if looping { "1" } else { "0" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_urls_and_normalizes() {
        for input in [
            "https://www.youtube.com/watch?v=M7lc1UVf-VE&t=3",
            "https://youtu.be/M7lc1UVf-VE",
            "https://www.youtube.com/live/M7lc1UVf-VE?feature=share",
        ] {
            assert_eq!(video_id(input).unwrap(), "M7lc1UVf-VE");
            assert_eq!(canonical_url(input).unwrap(), "https://www.youtube.com/watch?v=M7lc1UVf-VE");
        }
    }

    #[test]
    fn rejects_untrusted_or_invalid_ids() {
        for input in [
            "https://youtube.com.evil.test/watch?v=M7lc1UVf-VE",
            "https://www.youtube.com/watch?v=bad",
            "https://www.youtube.com/watch?v=M7lc1UVf-VE%3C",
            "https://youtu.be/M7lc1UVf-VE/extra",
            "file:///watch?v=M7lc1UVf-VE",
        ] {
            assert!(video_id(input).is_err(), "{input}");
        }
    }
}
