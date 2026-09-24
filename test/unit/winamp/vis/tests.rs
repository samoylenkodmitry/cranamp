use super::*;

#[test]
fn a_click_steps_through_the_analyzer_the_oscilloscope_and_nothing() {
    assert_eq!(VisMode::Analyzer.next(), VisMode::Oscilloscope);
    assert_eq!(VisMode::Oscilloscope.next(), VisMode::Off);
    assert_eq!(VisMode::Off.next(), VisMode::Analyzer);
    for mode in [VisMode::Analyzer, VisMode::Oscilloscope, VisMode::Off] {
        assert_eq!(VisMode::parse(mode.name()), Some(mode));
    }
    assert_eq!(VisMode::parse("fire"), None);
}

#[test]
fn a_bar_rises_at_once_and_falls_back_slowly_leaving_its_peak() {
    let mut analyzer = Analyzer::default();
    let mut levels = [0.0; BARS];
    levels[0] = 1.0;
    analyzer.advance(&levels, 1);
    let rows: Vec<_> = analyzer
        .runs()
        .into_iter()
        .filter(|run| run.0 == 0)
        .collect();
    assert_eq!(rows.len(), HEIGHT, "a full bar and no peak above it");
    assert_eq!(rows[0], (0, 0, 3, 2), "the top row is colour 2");
    assert_eq!(rows[HEIGHT - 1], (0, 15, 3, 17), "the bottom row colour 17");

    analyzer.advance(&[0.0; BARS], 8);
    let bar: Vec<_> = analyzer
        .runs()
        .into_iter()
        .filter(|run| run.0 == 0 && run.3 != PEAK)
        .collect();
    assert_eq!(
        bar.len(),
        10,
        "eight frames at 12/16 a frame take six rows off"
    );
    assert!(
        analyzer
            .runs()
            .iter()
            .any(|run| run.0 == 0 && run.3 == PEAK && run.1 == 0),
        "the peak is still at the top"
    );

    analyzer.advance(&[0.0; BARS], 120);
    assert!(analyzer.runs().is_empty(), "everything has fallen back");
}

#[test]
fn the_bars_are_winamps_thick_bands() {
    let mut analyzer = Analyzer::default();
    analyzer.advance(&[0.5; BARS], 1);
    let xs: Vec<usize> = analyzer
        .runs()
        .iter()
        .filter(|run| run.1 == HEIGHT - 1)
        .map(|run| run.0)
        .collect();
    assert_eq!(xs, (0..BARS).map(|bar| bar * 4).collect::<Vec<_>>());
    assert!(analyzer.runs().iter().all(|run| run.2 == 3));
}

#[test]
fn the_oscilloscope_joins_its_columns_and_colours_rows_by_their_distance_from_the_middle() {
    let flat = oscilloscope_runs(&[0.0; 576]);
    assert_eq!(flat.len(), 75, "a silent wave is one row across");
    assert!(flat.iter().all(|run| run.1 == 8 && run.3 == 19));

    let edge = oscilloscope_runs(&[1.0; 576]);
    assert!(edge.iter().all(|run| run.1 == 0 && run.3 == 21));

    let mut wave = vec![1.0; 288];
    wave.extend(vec![-1.0; 288]);
    let runs = oscilloscope_runs(&wave);
    let jump: Vec<_> = runs.iter().filter(|run| run.0 == 38).collect();
    assert_eq!(
        jump.len(),
        HEIGHT,
        "a jump from top to bottom is drawn as a line"
    );
    assert!(oscilloscope_runs(&[]).is_empty());
}

#[test]
fn the_field_is_dotted_on_every_other_pixel_of_every_other_row() {
    let dots: Vec<_> = dots().collect();
    assert_eq!(dots.len(), 38 * 8);
    assert_eq!(dots[0], (0, 1));
    assert!(dots.iter().all(|(x, y)| x % 2 == 0 && y % 2 == 1));
}
