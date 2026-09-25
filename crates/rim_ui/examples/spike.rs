//! Spike (0162): can taffy + cosmic-text + a Luau VM build a HUD-sized tree
//! inside the 1 ms budget? Measures each stage separately.
use mlua::{Lua, Table, Value};
use rim_ui::text::Text;
use std::time::Instant;
use taffy::prelude::*;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

fn main() {
    let t = Instant::now();
    let mut text = Text::new(None, &[]).expect("fonts");
    println!(
        "font load: {:.1} ms  ({} from {}, {} fallback faces)",
        ms(t),
        text.info.family,
        text.info.source,
        text.info.fallback_faces
    );

    let labels: Vec<String> = (0..300).map(|i| format!("Colonist {i} · hauling · 72%")).collect();
    let t = Instant::now();
    for l in &labels {
        text.shape(l, 13.0, 400, None);
    }
    println!("shape 300 labels, cold: {:.3} ms", ms(t));
    let t = Instant::now();
    for l in &labels {
        text.shape(l, 13.0, 400, None);
    }
    println!("shape 300 labels, cached: {:.3} ms", ms(t));
    let t = Instant::now();
    let mut quads = 0;
    for (i, l) in labels.iter().enumerate() {
        quads += text.quads(l, 13.0, 400, None, 10.0, i as f32 * 18.0).len();
    }
    println!("glyph quads, cold (rasterise {} glyph quads): {:.3} ms", quads, ms(t));
    let t = Instant::now();
    for (i, l) in labels.iter().enumerate() {
        text.quads(l, 13.0, 400, None, 10.0, i as f32 * 18.0);
    }
    println!("glyph quads, cached: {:.3} ms", ms(t));
    for s in ["Café déjà vu — naïve", "Ελληνικά", "日本語のテキスト", "العربية", "🔥🏠"]
    {
        let n = text.quads(s, 13.0, 400, None, 0.0, 0.0).len();
        println!("  fallback '{s}': {n} glyphs");
    }

    // Luau builds the tree; Rust converts it; taffy lays it out.
    let lua = Lua::new();
    lua.load(
        r#"
        function build(n)
          local rows = {}
          for i = 1, n do
            rows[i] = { kind = "row", gap = 4, { kind = "text", text = "Colonist " .. i }, { kind = "text", text = "hauling" } }
          end
          return { kind = "col", gap = 2, table.unpack(rows) }
        end
    "#,
    )
    .exec()
    .unwrap();
    let build: mlua::Function = lua.globals().get("build").unwrap();

    struct N {
        text: Option<String>,
        children: Vec<N>,
    }
    fn convert(t: &Table) -> N {
        let text = t.get::<Option<String>>("text").unwrap();
        let children = t.sequence_values::<Table>().map(|c| convert(&c.unwrap())).collect();
        N { text, children }
    }
    let mut times = (0.0, 0.0, 0.0);
    let frames = 200;
    for _ in 0..frames {
        let t = Instant::now();
        let tree: Table = build.call(100).unwrap();
        times.0 += ms(t);
        let t = Instant::now();
        let root = convert(&tree);
        times.1 += ms(t);

        let t = Instant::now();
        let mut taffy: TaffyTree<Option<(f32, f32)>> = TaffyTree::new();
        fn add(taffy: &mut TaffyTree<Option<(f32, f32)>>, n: &N, text: &mut Text) -> NodeId {
            if let Some(s) = &n.text {
                let sh = text.shape(s, 13.0, 400, None);
                return taffy.new_leaf_with_context(Style::default(), Some((sh.width, sh.height))).unwrap();
            }
            let kids: Vec<NodeId> = n.children.iter().map(|c| add(taffy, c, text)).collect();
            taffy
                .new_with_children(
                    Style { flex_direction: FlexDirection::Column, gap: Size::length(4.0), ..Default::default() },
                    &kids,
                )
                .unwrap()
        }
        let root_id = add(&mut taffy, &root, &mut text);
        taffy
            .compute_layout_with_measure(root_id, Size::MAX_CONTENT, |_input, _id, ctx, _style| {
                let s = ctx.and_then(|c| *c).unwrap_or((0.0, 0.0));
                taffy::LayoutOutput::from_outer_size(Size { width: s.0, height: s.1 })
            })
            .unwrap();
        times.2 += ms(t);
        let _ = Value::Nil;
    }
    let f = frames as f64;
    println!(
        "300-node tree per frame: luau build {:.3} ms, convert {:.3} ms, layout {:.3} ms, total {:.3} ms",
        times.0 / f,
        times.1 / f,
        times.2 / f,
        (times.0 + times.1 + times.2) / f
    );
}
