use super::*;
use cranpose_coroflow::StateFlowCollect;

#[composable]
pub(super) fn MetadataEffect(state: MutableState<WinampState>) {
    let _revision = crate::library::revision().collectAsState().get();
    let playback = cranpose_services::rememberPlaybackState().get();
    let ready = matches!(
        playback,
        cranpose_services::PlaybackState::Playing | cranpose_services::PlaybackState::Paused
    );
    let snapshot = state.get();
    let track = snapshot
        .current_index
        .and_then(|index| snapshot.playlist.get(index));
    let item = track.and_then(audio::media_item);
    let artwork = track.and_then(|track| crate::library::record(track).artwork);
    cranpose_core::LaunchedEffectAsync((item.clone(), artwork.clone(), ready), move |_| {
        Box::pin(async move {
            let Some(mut item) = item.filter(|_| ready) else {
                return;
            };
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            let cover_url = artwork.as_deref().and_then(crate::library::artwork_uri);
            if let Some(key) = artwork {
                item.metadata.artwork = crate::library::encoded_artwork(key).await;
            }
            #[cfg(target_os = "windows")]
            let cover_url = if let Some(artwork) = item.metadata.artwork.clone() {
                crate::system_media::prepare_cover(artwork).await
            } else {
                None
            };
            // A cover may finish loading after the user has selected another
            // track. Never attach that cover to the new media session.
            if cranpose_services::current_media_item()
                .is_some_and(|current| current.uri == item.uri)
            {
                #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
                crate::system_media::cover_changed(
                    item.uri.clone(),
                    if item.metadata.artwork.is_some() {
                        cover_url
                    } else {
                        None
                    },
                );
                cranpose_services::set_media_metadata(item.metadata);
            }
        })
    });
}
