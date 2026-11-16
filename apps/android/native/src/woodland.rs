//! A botanical word-search page using the same native controls as Garden.
use super::*;
use bevy::ui::widget::NodeImageMode;
use home::{HomeArt, label, node, pill};
const INK: Color = Color::srgb(0.055, 0.20, 0.14);
const CREAM: Color = Color::srgb(0.99, 0.94, 0.81);
const GOLD: Color = Color::srgb(0.76, 0.59, 0.30);
fn in_line(cell: [usize; 2], start: [usize; 2], end: [usize; 2]) -> bool {
    let dr = end[0] as isize - start[0] as isize;
    let dc = end[1] as isize - start[1] as isize;
    if dr != 0 && dc != 0 && dr.abs() != dc.abs() {
        return cell == start;
    }
    (0..=dr.abs().max(dc.abs())).any(|i| {
        cell == [
            (start[0] as isize + i * dr.signum()) as usize,
            (start[1] as isize + i * dc.signum()) as usize,
        ]
    })
}

pub(super) fn draw(c: &mut Commands, root: Entity, p: &mut Pocket, canvas: &Canvas, art: &HomeArt) {
    let Some(g) = p.saved.game.clone() else {
        return;
    };
    let h = canvas.height;
    c.spawn((
        ImageNode {
            image: art.garden.clone(),
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        node([0., 0., 480., h.max(980.)]),
        ChildOf(root),
        bevy::ui::FocusPolicy::Pass,
    ));
    if !p.saved.reduced_motion {
        garden::glints(c, root, art, g.won);
    }
    beacon::draw(c, root, p, art, [18., 20., 55., 52.]);
    label(
        c,
        root,
        "Woodland Expedition",
        [78., 22., 324., 48.],
        28.,
        CREAM,
        &art.display,
        true,
    );
    pill(
        c,
        root,
        if p.saved.muted { "Off" } else { "On" },
        [406., 20., 57., 52.],
        Action::Mute,
        CREAM,
        INK,
        &art.body,
    );
    label(
        c,
        root,
        "Sound",
        [406., 76., 57., 24.],
        14.,
        CREAM,
        &art.body,
        true,
    );
    label(
        c,
        root,
        "A little word hunt",
        [78., 77., 324., 32.],
        22.,
        CREAM,
        &art.body,
        true,
    );
    label(
        c,
        root,
        "Slide your finger along a word.",
        [22., 124., 436., 32.],
        21.,
        CREAM,
        &art.body,
        true,
    );
    let top = 169.;
    let size = (h - 470.).clamp(320., 436.);
    let left = (480. - size) * 0.5;
    c.spawn((
        Node {
            border: UiRect::all(px(3.)),
            ..node([left, top, size, size])
        },
        BackgroundColor(INK),
        BorderColor::all(GOLD),
        BorderRadius::all(px(20.)),
        ChildOf(root),
    ));
    let rows = g.rows();
    let cols = g.cols();
    let pad = 10.;
    let gap = 2.;
    let cell = (size - pad * 2.) / rows.max(cols) as f32;
    for r in 0..rows {
        for col in 0..cols {
            let selected = p
                .word_preview
                .map_or(g.selected == Some([r, col]), |(start, end)| {
                    in_line([r, col], start, end)
                });
            let found = g
                .found
                .iter()
                .any(|word| coords(&word["cells"]).contains(&[r, col]));
            let letter = g.puzzle.data["grid"][r][col].as_str().unwrap_or("");
            garden::key(
                c,
                root,
                letter,
                [
                    left + pad + col as f32 * cell,
                    top + pad + r as f32 * cell,
                    cell - gap,
                    cell - gap,
                ],
                (!g.won && !p.busy).then_some(Action::Tap([r, col])),
                &art.body,
                (cell * 0.63).clamp(16., 29.),
                if found {
                    Color::srgb(0.72, 0.85, 0.52)
                } else {
                    CREAM
                },
                INK,
                selected,
            );
        }
    }
    let list_y = top + size + 16.;
    let words = g.puzzle.data["words"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    label(
        c,
        root,
        format!("Field journal · {} / {} found", g.found.len(), words.len()),
        [22., list_y, 436., 32.],
        24.,
        CREAM,
        &art.display,
        true,
    );
    for (i, word) in words.iter().enumerate() {
        let found = g.found.iter().any(|f| f["word"] == *word);
        let text = format!(
            "{}{}",
            if found { "Found: " } else { "" },
            word.as_str().unwrap_or("")
        );
        let rect = [
            22. + (i % 3) as f32 * 146.,
            list_y + 41. + (i / 3) as f32 * 39.,
            140.,
            34.,
        ];
        c.spawn((
            node(rect),
            BackgroundColor(if found {
                Color::srgb(0.72, 0.85, 0.52)
            } else {
                CREAM
            }),
            BorderRadius::all(px(10.)),
            ChildOf(root),
        ));
        label(
            c,
            root,
            text,
            [rect[0] + 3., rect[1] + 4., 134., 30.],
            if found { 15. } else { 18. },
            INK,
            &art.body,
            true,
        );
    }
    let y = list_y + 47. + words.len().div_ceil(3) as f32 * 39.;
    garden::key(
        c,
        root,
        "Undo",
        [22., y, 101., 54.],
        (!p.history.is_empty() && !p.busy && !g.won).then_some(Action::Undo),
        &art.body,
        22.,
        CREAM,
        INK,
        false,
    );
    garden::key(
        c,
        root,
        "Hint",
        [134., y, 101., 54.],
        (!p.busy && !g.won).then_some(Action::Hint),
        &art.body,
        22.,
        CREAM,
        INK,
        false,
    );
    label(
        c,
        root,
        if g.won {
            "Lovely work!"
        } else if p.busy {
            "Please wait…"
        } else {
            "Keep exploring"
        },
        [247., y + 12., 211., 54.],
        23.,
        CREAM,
        &art.display,
        true,
    );
    pill(
        c,
        root,
        "New word hunt",
        [if g.won { 246. } else { 134. }, y + 72., 212., 52.],
        Action::New,
        CREAM,
        INK,
        &art.display,
    );
    if g.won {
        pill(
            c,
            root,
            "My Garden",
            [22., y + 72., 212., 52.],
            Action::MyGarden,
            CREAM,
            INK,
            &art.display,
        );
    }
    let tip = if !p.message.is_empty() {
        p.message.as_str()
    } else if g.won {
        "Every word discovered. A lovely little expedition!"
    } else if g.selected.is_some() {
        "Now tap the last letter. Words can run diagonally or backwards."
    } else {
        "Find the journal words in straight lines. Take your time."
    };
    c.spawn((
        node([22., y + 143., 436., 92.]),
        BackgroundColor(INK.with_alpha(0.88)),
        BorderRadius::all(px(13.)),
        ChildOf(root),
    ));
    label(
        c,
        root,
        tip,
        [34., y + 151., 412., 80.],
        21.,
        CREAM,
        &art.body,
        true,
    );
    p.content_height = h.max(y + 255.);
    p.scroll = p.scroll.clamp(0., (p.content_height - h).max(0.));
}
