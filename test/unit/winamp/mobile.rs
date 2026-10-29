use super::*;

#[test]
fn library_is_second_and_all_mobile_destinations_are_exclusive() {
    let mut state = WinampState::default();
    for (index, tab) in [
        Destination::Player,
        Destination::Library,
        Destination::Studio,
        Destination::Settings,
    ]
    .into_iter()
    .enumerate()
    {
        tab.select(&mut state);
        assert_eq!(tab.index(), index);
        assert_eq!(Destination::from_state(&state), tab);
        assert_eq!(state.library_open, tab == Destination::Library);
        assert_eq!(state.studio_open, tab == Destination::Studio);
        assert_eq!(state.settings_open, tab == Destination::Settings);
    }
}

#[test]
fn expanded_transport_keeps_the_original_button_centers() {
    for scale in [0.7, 1.0, 1.43, 2.0, 3.0] {
        for (index, (sprite, _, x, _)) in TRANSPORT.iter().enumerate() {
            let center = (x + sprite.2 * 0.5) * scale;
            assert_eq!(nearest_transport(center, scale), index);
            if index + 1 < TRANSPORT.len() {
                let next = TRANSPORT[index + 1];
                let midpoint = (center + (next.2 + next.0 .2 * 0.5) * scale) / 2.0;
                assert_eq!(nearest_transport(midpoint - 0.01, scale), index);
                assert_eq!(nearest_transport(midpoint + 0.01, scale), index + 1);
            }
        }
    }
}

#[test]
fn accessible_slider_commits_a_change_like_a_complete_drag() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let change = events.clone();
    let drag = events.clone();
    let configure = player_slider_semantics(
        "Volume".into(),
        0.4,
        Rc::new(move |value| change.borrow_mut().push(format!("value:{value}"))),
        Rc::new(move |active| drag.borrow_mut().push(format!("drag:{active}"))),
    );
    let mut config = cranpose_foundation::SemanticsConfiguration::default();
    configure(&mut config);
    assert_eq!(config.content_description.as_deref(), Some("Volume"));
    assert!(config.progress.is_some());
    assert!(config.set_progress.unwrap().invoke(1.5));
    assert_eq!(&*events.borrow(), &["drag:true", "value:1", "drag:false"]);
}
