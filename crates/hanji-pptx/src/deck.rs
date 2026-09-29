//! What a deck's masters, layouts and notes master give the text: layout
//! names, the slots each layout's placeholders make, and the bullets each
//! slot's paragraphs inherit.

use serde::{Deserialize, Serialize};

use hanji_core::Part;
use hanji_package::xml::{self, Element};
use hanji_package::{opc, package};

use crate::pml::*;

/// One slot of a layout: a placeholder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotInfo {
    pub name: String,
    /// Placeholder type (`obj` when the layout gives none) and index.
    pub ty: String,
    pub idx: u32,
    /// The layout's `p:ph`, which a new slide's placeholder copies.
    pub ph: String,
    /// The layout shape's name (`Title 1`), which a new placeholder takes.
    pub shape_name: String,
    pub class: TextClass,
    /// Per level, the bullet the slot's paragraphs inherit: the layout's list
    /// style over the master placeholder's over the master text style.
    pub bullets: [Bu; 9],
    /// The box a slide placeholder of this slot inherits: the layout
    /// placeholder's own, else its master placeholder's.
    #[serde(default)]
    pub geom: Option<hanji_format::Geom>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutInfo {
    /// The name the text writes (`layout: Name`), unique in the deck.
    pub name: String,
    pub part: String,
    pub slots: Vec<SlotInfo>,
    /// Bullets of text in shapes that are not placeholders (the master's `p:otherStyle`).
    pub other: [Bu; 9],
}

/// The notes master: what a notes page's text inherits, and what a new notes page is made from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesMaster {
    pub part: String,
    pub bullets: [Bu; 9],
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deck {
    pub layouts: Vec<LayoutInfo>,
    pub notes: Option<NotesMaster>,
    /// The slide size in EMU (`p:sldSz`).
    #[serde(default)]
    pub size: Option<(i64, i64)>,
}

impl LayoutInfo {
    pub fn slot(&self, name: &str) -> Option<&SlotInfo> {
        self.slots.iter().find(|s| s.name == name)
    }

    /// [`slot`](Self::slot), or the error the export gives for a slot the layout lacks.
    pub fn need_slot(&self, name: &str) -> Result<&SlotInfo, String> {
        self.slot(name).ok_or_else(|| format!("::{name}:: is not a slot of layout {:?}", self.name))
    }
}

impl Deck {
    pub fn layout(&self, name: &str) -> Option<&LayoutInfo> {
        self.layouts.iter().find(|l| l.name == name)
    }

    pub fn layout_of_part(&self, part: &str) -> Option<&LayoutInfo> {
        self.layouts.iter().find(|l| l.part == part)
    }

    pub fn read(parts: &[Part], pres_part: &str, pres: &Element) -> Result<Deck, String> {
        let size = pres.child("p:sldSz").and_then(|s| Some((s.get("cx")?.parse().ok()?, s.get("cy")?.parse().ok()?)));
        let mut deck = Deck { size, ..Default::default() };
        let ids: Vec<String> = pres
            .child("p:sldMasterIdLst")
            .map(|l| l.elements().filter_map(|e| e.get("r:id")).collect())
            .unwrap_or_default();
        for id in ids {
            let Some(mpart) = opc::target_of(parts, pres_part, &id) else { continue };
            let master = parse(parts, &mpart)?;
            let layouts: Vec<String> = master
                .child("p:sldLayoutIdLst")
                .map(|l| l.elements().filter_map(|e| e.get("r:id")).collect())
                .unwrap_or_default();
            let styles = master.child("p:txStyles");
            let text_style = |n: &str| levels_of(styles.and_then(|s| s.child(n)));
            let (title, body, other) =
                (text_style("p:titleStyle"), text_style("p:bodyStyle"), text_style("p:otherStyle"));
            let mphs = placeholders(&master);
            for lid in layouts {
                let Some(lpart) = opc::target_of(parts, &mpart, &lid) else { continue };
                let layout = parse(parts, &lpart)?;
                let stem = lpart.rsplit('/').next().unwrap_or(&lpart).trim_end_matches(".xml").to_string();
                let base = layout.child("p:cSld").and_then(|c| c.get("name")).filter(|n| !n.is_empty()).unwrap_or(stem);
                let name = unique(&base, |n| deck.layout(n).is_some());
                let mut slots = vec![];
                let phs = placeholders(&layout);
                let body_like: Vec<usize> = (0..phs.len()).filter(|&k| is_body_type(&phs[k].ty)).collect();
                let mut body_names: Vec<String> = match body_like.len() {
                    1 => vec!["body".into()],
                    2 => {
                        let (a, b) = (&phs[body_like[0]], &phs[body_like[1]]);
                        if b.x.is_some() && a.x.is_some() && b.x < a.x {
                            vec!["right".into(), "left".into()]
                        } else {
                            vec!["left".into(), "right".into()]
                        }
                    }
                    n => (1..=n).map(|k| if k == 1 { "body".into() } else { format!("body{k}") }).collect(),
                };
                body_names.reverse();
                for ph in &phs {
                    let base = match slot_of_type(&ph.ty) {
                        Some(n) => n.to_string(),
                        None => body_names.pop().unwrap_or_else(|| "body".into()),
                    };
                    let name = unique(&base, |n| slots.iter().any(|s: &SlotInfo| s.name == n));
                    let class = class_of_type(&ph.ty);
                    let master_ph = mphs.iter().find(|m| match class {
                        TextClass::Title => class_of_type(&m.ty) == TextClass::Title,
                        TextClass::Other => m.ty == ph.ty,
                        _ => is_body_type(&m.ty),
                    });
                    let base_bu = match class {
                        TextClass::Title => title,
                        TextClass::Other => other,
                        _ => body,
                    };
                    let bullets = over(ph.lst, over(master_ph.map_or([Bu::Unset; 9], |m| m.lst), base_bu));
                    let geom = ph.geom.or(master_ph.and_then(|m| m.geom));
                    slots.push(SlotInfo {
                        name,
                        ty: ph.ty.clone(),
                        idx: ph.idx,
                        ph: ph.ph.clone(),
                        shape_name: ph.name.clone(),
                        class,
                        bullets,
                        geom,
                    });
                }
                deck.layouts.push(LayoutInfo { name, part: lpart, slots, other });
            }
        }
        let nm = pres.child("p:notesMasterIdLst").and_then(|l| l.elements().next()).and_then(|e| e.get("r:id"));
        if let Some(npart) = nm.and_then(|id| opc::target_of(parts, pres_part, &id)) {
            let master = parse(parts, &npart)?;
            let style = levels_of(master.child("p:notesStyle"));
            let body = placeholders(&master).into_iter().find(|p| p.ty == "body");
            deck.notes =
                Some(NotesMaster { part: npart, bullets: over(body.map_or([Bu::Unset; 9], |b| b.lst), style) });
        }
        Ok(deck)
    }
}

/// `base`, or `base (2)`, `base (3)`, … — the first that is not taken.
fn unique(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    let digit = base.ends_with(|c: char| c.is_ascii_digit());
    (2..)
        .map(|k| {
            if base.chars().all(|c| c.is_ascii_lowercase()) && !digit {
                format!("{base}{k}")
            } else {
                format!("{base} ({k})")
            }
        })
        .find(|n| !taken(n))
        .unwrap()
}

fn parse(parts: &[Part], name: &str) -> Result<Element, String> {
    let data = package::get(parts, name).ok_or_else(|| format!("no {name}"))?;
    Ok(xml::parse(data).map_err(|e| format!("{name}: {e}"))?.root)
}

pub(crate) struct Ph {
    pub ty: String,
    pub idx: u32,
    pub ph: String,
    pub name: String,
    pub x: Option<i64>,
    pub lst: [Bu; 9],
    pub geom: Option<hanji_format::Geom>,
}

/// The placeholders of a master's or layout's shape tree, in order.
pub(crate) fn placeholders(root: &Element) -> Vec<Ph> {
    let Some(tree) = root.child("p:cSld").and_then(|c| c.child("p:spTree")) else { return vec![] };
    tree.elements()
        .filter_map(|sh| {
            let (ty, idx, ph) = placeholder(sh)?;
            let x = sh
                .child("p:spPr")
                .and_then(|s| s.child("a:xfrm"))
                .and_then(|x| x.child("a:off"))
                .and_then(|o| o.get("x"))
                .and_then(|v| v.parse().ok());
            let lst = levels_of(sh.child("p:txBody").and_then(|t| t.child("a:lstStyle")));
            let name = c_nv_pr(sh).and_then(|c| c.get("name")).unwrap_or_default();
            Some(Ph { ty, idx, ph: ph.to_xml(), name, x, lst, geom: crate::geom::own(sh) })
        })
        .collect()
}
