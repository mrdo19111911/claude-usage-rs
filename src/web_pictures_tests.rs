// The step pictures of the Gemini dialog (docs/specs/gemini.md, "Step pictures"): real
// screenshots built into the program, served as PNG, shown under each step and in help.

use super::{asset, image};

const PICS: [&str; 6] = ["/img/gem-1-signin.png", "/img/gem-2-create.png", "/img/gem-3-project.png",
                         "/img/gem-4-newproject.png", "/img/gem-5-createkey.png", "/img/gem-6-copy.png"];

#[test]
fn each_picture_is_a_small_png() {
    for p in PICS {
        let png = image(p).unwrap_or_else(|| panic!("{p} served"));
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "{p} is a PNG");
        assert!(png.len() < 150_000, "{p} is {} bytes", png.len());
    }
    assert!(image("/img/nothing.png").is_none());
    assert!(image("/").is_none(), "pages are not pictures");
}

#[test]
fn each_step_shows_its_picture_with_alt_text() {
    let p = asset("/").unwrap().0;
    let dlg = &p[p.find("<dialog id=\"gemDlg\"").unwrap()..p.find("</dialog>").unwrap()];
    let steps: Vec<&str> = dlg.split("<li>").skip(1).collect();
    // Sign in; make a code (four pictures); Copy.
    let at = [0, 1, 1, 1, 1, 2];
    for (k, pic) in at.into_iter().zip(PICS) {
        let step = steps[k];
        let src = format!("src=\"{}\"", &pic[1..]);
        let i = step.find(&src).unwrap_or_else(|| panic!("{pic} in its step"));
        let tag = &step[step[..i].rfind("<img").unwrap()..i + step[i..].find('>').unwrap()];
        assert!(tag.contains("alt=\"") && !tag.contains("alt=\"\""), "{tag}: alt text");
        assert!(tag.contains("class=\"shot\""), "{tag}: shrinks to fit");
    }
}

#[test]
fn help_shows_the_same_pictures() {
    let help = asset("/help.html").unwrap().0;
    for p in PICS {
        assert!(help.contains(&format!("src=\"{}\"", &p[1..])), "help shows {p}");
    }
    let css = asset("/style.css").unwrap().0;
    assert!(css.contains(".shot {") && css.contains("max-width: 100%"), "pictures fit the dialog");
}
