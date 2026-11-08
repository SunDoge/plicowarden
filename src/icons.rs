use plico::SvgAsset;
pub fn get(name: &str) -> &'static SvgAsset {
    match name {
        "grid" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/grid.svg");
            &ICON
        }
        "star" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/star.svg");
            &ICON
        }
        "key" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/key.svg");
            &ICON
        }
        "card" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/card.svg");
            &ICON
        }
        "person" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/person.svg");
            &ICON
        }
        "note" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/note.svg");
            &ICON
        }
        "terminal" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/terminal.svg");
            &ICON
        }
        "folder" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/folder.svg");
            &ICON
        }
        "shield" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/shield.svg");
            &ICON
        }
        "clock" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/clock.svg");
            &ICON
        }
        "sliders" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/sliders.svg");
            &ICON
        }
        "search" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/search.svg");
            &ICON
        }
        "sort" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/sort.svg");
            &ICON
        }
        "eye" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/eye.svg");
            &ICON
        }
        "copy" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/copy.svg");
            &ICON
        }
        "external" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/external.svg");
            &ICON
        }
        "lock" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/lock.svg");
            &ICON
        }
        "plus" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/plus.svg");
            &ICON
        }
        "sidebar" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/sidebar.svg");
            &ICON
        }
        "chevron" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/chevron.svg");
            &ICON
        }
        "check" => {
            static ICON: SvgAsset = plico::svg_asset!("../assets/icons/check.svg");
            &ICON
        }
        _ => get("shield"),
    }
}
