use super::{
    pane_came_back_to_its_slot, pane_left_the_stack, WinampDock, WinampPane, MAIN_HEIGHT,
    MAIN_WIDTH,
};
use cranpose_ui::Point;

#[test]
fn a_pane_stays_in_the_stack_until_the_title_has_really_been_carried_off() {
    assert!(
        !pane_left_the_stack(Point::new(4.0, 6.0)),
        "the press that means to move the whole stack wanders a few pixels, and tearing \
         a pane off every time it does would make the window impossible to drag"
    );
    assert!(pane_left_the_stack(Point::new(0.0, 40.0)));
    assert!(pane_left_the_stack(Point::new(-40.0, 0.0)));
}

#[test]
fn a_pane_docks_when_its_top_edge_meets_the_bottom_of_the_stack() {
    let slot = Point::new(140.0, 497.0);
    assert!(
        pane_came_back_to_its_slot(Point::new(153.0, 497.0), slot),
        "the group snaps a dragged window to the edge it meets, which lands the top edge \
         exactly and leaves it sitting a little to one side; asking for the sideways pixel \
         as well would mean the pane almost never goes back in"
    );
    assert!(pane_came_back_to_its_slot(slot, slot));
    assert!(
        pane_came_back_to_its_slot(Point::new(255.0, 540.0), Point::new(240.0, 550.0)),
        "a pane let go ten pixels short of the stack was meant for it; the group's own \
         snap never reaches that far, so the stack has to"
    );
    assert!(
        !pane_came_back_to_its_slot(Point::new(140.0, 497.0 + MAIN_HEIGHT), slot),
        "a pane a whole pane's height below the stack was put down, not docked"
    );
    assert!(
        !pane_came_back_to_its_slot(Point::new(140.0 + MAIN_WIDTH, 497.0), slot),
        "a pane beside the stack rather than under it is not in it"
    );
}

#[test]
fn a_torn_pane_is_the_only_one_that_leaves_the_main_window() {
    let mut dock = WinampDock::default();
    assert!(dock.docked(WinampPane::Equalizer));
    assert!(dock.docked(WinampPane::Playlist));
    dock.tear(WinampPane::Equalizer);
    assert!(!dock.docked(WinampPane::Equalizer));
    assert!(
        dock.docked(WinampPane::Playlist),
        "pulling one pane out leaves the rest of the stack where it was"
    );
    dock.tear(WinampPane::Equalizer);
    dock.dock(WinampPane::Equalizer);
    assert!(
        dock.docked(WinampPane::Equalizer),
        "a pane torn twice is still one pane, so docking it once puts it back"
    );
}
