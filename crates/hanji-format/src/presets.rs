//! DrawingML's preset shape names (`ST_ShapeType`), which the text names as they are.

/// DrawingML's preset shapes (`ST_ShapeType`): a picture's `mask`.
#[rustfmt::skip]
pub const PRESET_SHAPES: &[&str] = &[
    "line", "lineInv", "triangle", "rtTriangle", "rect", "diamond", "parallelogram", "trapezoid",
    "nonIsoscelesTrapezoid", "pentagon", "hexagon", "heptagon", "octagon", "decagon", "dodecagon", "star4", "star5",
    "star6", "star7", "star8", "star10", "star12", "star16", "star24", "star32", "roundRect", "round1Rect",
    "round2SameRect", "round2DiagRect", "snipRoundRect", "snip1Rect", "snip2SameRect", "snip2DiagRect", "plaque",
    "ellipse", "teardrop", "homePlate", "chevron", "pieWedge", "pie", "blockArc", "donut", "noSmoking", "rightArrow",
    "leftArrow", "upArrow", "downArrow", "stripedRightArrow", "notchedRightArrow", "bentUpArrow", "leftRightArrow",
    "upDownArrow", "leftUpArrow", "leftRightUpArrow", "quadArrow", "leftArrowCallout", "rightArrowCallout",
    "upArrowCallout", "downArrowCallout", "leftRightArrowCallout", "upDownArrowCallout", "quadArrowCallout",
    "bentArrow", "uturnArrow", "circularArrow", "leftCircularArrow", "leftRightCircularArrow", "curvedRightArrow",
    "curvedLeftArrow", "curvedUpArrow", "curvedDownArrow", "swooshArrow", "cube", "can", "lightningBolt", "heart",
    "sun", "moon", "smileyFace", "irregularSeal1", "irregularSeal2", "foldedCorner", "bevel", "frame", "halfFrame",
    "corner", "diagStripe", "chord", "arc", "leftBracket", "rightBracket", "leftBrace", "rightBrace", "bracketPair",
    "bracePair", "straightConnector1", "bentConnector2", "bentConnector3", "bentConnector4", "bentConnector5",
    "curvedConnector2", "curvedConnector3", "curvedConnector4", "curvedConnector5", "callout1", "callout2",
    "callout3", "accentCallout1", "accentCallout2", "accentCallout3", "borderCallout1", "borderCallout2",
    "borderCallout3", "accentBorderCallout1", "accentBorderCallout2", "accentBorderCallout3", "wedgeRectCallout",
    "wedgeRoundRectCallout", "wedgeEllipseCallout", "cloudCallout", "cloud", "ribbon", "ribbon2", "ellipseRibbon",
    "ellipseRibbon2", "leftRightRibbon", "verticalScroll", "horizontalScroll", "wave", "doubleWave", "plus",
    "flowChartProcess", "flowChartDecision", "flowChartInputOutput", "flowChartPredefinedProcess",
    "flowChartInternalStorage", "flowChartDocument", "flowChartMultidocument", "flowChartTerminator",
    "flowChartPreparation", "flowChartManualInput", "flowChartManualOperation", "flowChartConnector",
    "flowChartPunchedCard", "flowChartPunchedTape", "flowChartSummingJunction", "flowChartOr", "flowChartCollate",
    "flowChartSort", "flowChartExtract", "flowChartMerge", "flowChartOfflineStorage", "flowChartOnlineStorage",
    "flowChartMagneticTape", "flowChartMagneticDisk", "flowChartMagneticDrum", "flowChartDisplay", "flowChartDelay",
    "flowChartAlternateProcess", "flowChartOffpageConnector", "actionButtonBlank", "actionButtonHome",
    "actionButtonHelp", "actionButtonInformation", "actionButtonForwardNext", "actionButtonBackPrevious",
    "actionButtonEnd", "actionButtonBeginning", "actionButtonReturn", "actionButtonDocument", "actionButtonSound",
    "actionButtonMovie", "gear6", "gear9", "funnel", "mathPlus", "mathMinus", "mathMultiply", "mathDivide",
    "mathEqual", "mathNotEqual", "cornerTabs", "squareTabs", "plaqueTabs", "chartX", "chartStar", "chartPlus",
];

/// Whether `name` is one of DrawingML's preset shapes.
pub fn is_preset(name: &str) -> bool {
    PRESET_SHAPES.contains(&name)
}

/// The preset names closest to a misspelt one (case-insensitive prefix or containment), at most five.
pub fn near_presets(name: &str) -> Vec<&'static str> {
    let l = name.to_ascii_lowercase();
    let mut out: Vec<&'static str> = PRESET_SHAPES
        .iter()
        .copied()
        .filter(|p| {
            let q = p.to_ascii_lowercase();
            !l.is_empty() && (q == l || q.starts_with(&l) || q.contains(&l) || l.contains(&q))
        })
        .collect();
    out.truncate(5);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_preset_of_st_shape_type() {
        assert_eq!(super::PRESET_SHAPES.len(), 187);
        let mut v = super::PRESET_SHAPES.to_vec();
        v.sort();
        v.dedup();
        assert_eq!(v.len(), 187);
    }
}
