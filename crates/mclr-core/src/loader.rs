use crate::mcver;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Loader {
    Fabric,
    Quilt,
    LegacyFabric,
    Forge,
    NeoForge,
    Cleanroom,
    LiteLoader,
    OptiFine,
    LabyMod,
}

impl Loader {
    /// Parses a loader label into a `Loader`.
    ///
    /// Lowercases the label, drops `' '`, `'_'`, `'-'`.
    /// First hit in order wins (longer names before names they contain):
    /// `legacyfabric, neoforge, quilt, fabric, cleanroom, forge, liteloader, optifine, labymod`.
    pub fn parse(label: &str) -> Option<Self> {
        let mut buf = [0u8; 64];
        let mut len = 0;
        for b in label.bytes() {
            if b != b' ' && b != b'_' && b != b'-' && len < buf.len() {
                buf[len] = b.to_ascii_lowercase();
                len += 1;
            }
        }
        let clean = std::str::from_utf8(&buf[..len]).ok()?;
        if clean.is_empty() {
            return None;
        }

        if clean.contains("legacyfabric") {
            Some(Loader::LegacyFabric)
        } else if clean.contains("neoforge") {
            Some(Loader::NeoForge)
        } else if clean.contains("quilt") {
            Some(Loader::Quilt)
        } else if clean.contains("fabric") {
            Some(Loader::Fabric)
        } else if clean.contains("cleanroom") {
            Some(Loader::Cleanroom)
        } else if clean.contains("forge") {
            Some(Loader::Forge)
        } else if clean.contains("liteloader") {
            Some(Loader::LiteLoader)
        } else if clean.contains("optifine") {
            Some(Loader::OptiFine)
        } else if clean.contains("labymod") {
            Some(Loader::LabyMod)
        } else {
            None
        }
    }

    /// Lowercase canonical identifier (e.g. `"fabric"`, `"neoforge"`).
    pub fn id(self) -> &'static str {
        match self {
            Loader::Fabric => "fabric",
            Loader::Quilt => "quilt",
            Loader::LegacyFabric => "legacyfabric",
            Loader::Forge => "forge",
            Loader::NeoForge => "neoforge",
            Loader::Cleanroom => "cleanroom",
            Loader::LiteLoader => "liteloader",
            Loader::OptiFine => "optifine",
            Loader::LabyMod => "labymod",
        }
    }

    /// Platform index identifier as understood by Modrinth / CurseForge.
    /// Returns `None` for flavours not indexed as platforms (`cleanroom`, `legacyfabric`, etc.).
    pub fn platform_id(self) -> Option<&'static str> {
        match self {
            Loader::Fabric => Some("fabric"),
            Loader::Quilt => Some("quilt"),
            Loader::Forge => Some("forge"),
            Loader::NeoForge => Some("neoforge"),
            Loader::LiteLoader => Some("liteloader"),
            _ => None,
        }
    }

    /// Whether this loader accepts mods. False only for `OptiFine` and `LabyMod`.
    pub fn is_modable(self) -> bool {
        !matches!(self, Loader::OptiFine | Loader::LabyMod)
    }

    /// Priority rank for sorting / primary loader selection.
    pub fn priority(self) -> u32 {
        match self {
            Loader::Fabric => 1,
            Loader::Quilt => 2,
            Loader::LegacyFabric => 3,
            Loader::NeoForge => 4,
            Loader::Forge => 5,
            Loader::Cleanroom => 6,
            Loader::LabyMod => 7,
            Loader::OptiFine => 8,
            Loader::LiteLoader => 9,
        }
    }
}

/// Computes the best loader priority across labels.
/// Returns 99 for empty slices, 50 for unknown-only labels.
pub fn best_priority(labels: &[String]) -> u32 {
    if labels.is_empty() {
        return 99;
    }
    let mut score = 99;
    for label in labels {
        let s = Loader::parse(label).map(|l| l.priority()).unwrap_or(50);
        if s < score {
            score = s;
        }
    }
    score
}

/// Returns true if any label parses to a modable loader.
pub fn is_modable_labels(labels: &[String]) -> bool {
    labels
        .iter()
        .any(|l| Loader::parse(l).is_some_and(Loader::is_modable))
}

/// Expands loader labels into mod targets (order-preserving, deduplicated).
///
/// NeoForge -> `neoforge` (+ `forge` on 1.20.1),
/// Quilt -> `quilt`, `fabric`,
/// Cleanroom -> `forge`,
/// LegacyFabric -> `fabric`,
/// others -> own id,
/// unparsable labels pass through trimmed + lowercased (blanks dropped).
pub fn expand_mod_loaders(labels: &[String], mc: &str) -> Vec<String> {
    let mc = mc.trim();
    let mut out: Vec<String> = Vec::with_capacity(labels.len() + 1);
    let mut push = |id: &str| {
        let id = id.trim().to_ascii_lowercase();
        if !id.is_empty() && !out.iter().any(|existing| existing == &id) {
            out.push(id);
        }
    };
    for label in labels {
        match Loader::parse(label) {
            Some(Loader::NeoForge) => {
                push("neoforge");
                if mc == "1.20.1" {
                    push("forge");
                }
            }
            Some(Loader::Quilt) => {
                push("quilt");
                push("fabric");
            }
            Some(Loader::Cleanroom) => push("forge"),
            Some(Loader::LegacyFabric) => push("fabric"),
            Some(loader) => push(loader.id()),
            None => {
                push(label);
            }
        }
    }
    out
}

/// Maps instance loader labels to canonical ids, keeping flavours `platform_id`
/// drops (Cleanroom, LegacyFabric) so [`expand_mod_loaders`] can map them.
/// Unknown labels pass through lowercased; blanks are dropped.
pub fn canonical_ids(labels: &[String]) -> Vec<String> {
    labels
        .iter()
        .filter_map(|label| {
            let lower = label.trim().to_ascii_lowercase();
            if lower.is_empty() {
                return None;
            }
            Some(
                Loader::parse(label)
                    .map(|l| l.id().to_string())
                    .unwrap_or(lower),
            )
        })
        .collect()
}

/// Returns supported loaders for the given Minecraft version in enum declaration order.
pub fn supported_for(mc: &str) -> Vec<Loader> {
    let mc = mc.trim();
    let is_standard_release =
        mc.starts_with("1.") || mc.chars().take_while(|c| c.is_ascii_digit()).count() >= 2;
    let drop = mcver::drop_of(mc);
    let mut supported = Vec::new();

    if drop >= 130 {
        supported.push(Loader::Fabric);
    }
    if drop >= 130 {
        supported.push(Loader::Quilt);
    }
    if (30..=130).contains(&drop) {
        supported.push(Loader::LegacyFabric);
    }
    if is_standard_release && mcver::compare(mc, "1.1").is_ge() {
        supported.push(Loader::Forge);
    }
    if is_standard_release && mcver::compare(mc, "1.20.1").is_ge() && mc != "1.20" && mc != "1.20.0"
    {
        supported.push(Loader::NeoForge);
    }
    if mc == "1.12.2" {
        supported.push(Loader::Cleanroom);
    }
    if is_standard_release
        && mcver::compare(mc, "1.5.2").is_ge()
        && mcver::compare(mc, "1.12.2").is_le()
    {
        supported.push(Loader::LiteLoader);
    }
    if mcver::compare(mc, "1.7.2").is_ge() {
        supported.push(Loader::OptiFine);
    }
    if is_standard_release && mcver::compare(mc, "1.8.9").is_ge() {
        supported.push(Loader::LabyMod);
    }

    supported
}

/// Incompatible loader pairs for the given Minecraft version.
pub fn incompatible_pairs(mc: &str) -> Vec<(Loader, Loader)> {
    let mc = mc.trim();
    let mut pairs = vec![
        (Loader::Fabric, Loader::Forge),
        (Loader::Fabric, Loader::NeoForge),
        (Loader::Forge, Loader::NeoForge),
        (Loader::Cleanroom, Loader::OptiFine),
        (Loader::Cleanroom, Loader::LiteLoader),
        (Loader::NeoForge, Loader::LiteLoader),
        (Loader::LabyMod, Loader::OptiFine),
        (Loader::LabyMod, Loader::LiteLoader),
        (Loader::LegacyFabric, Loader::LiteLoader),
    ];

    if mcver::compare(mc, "1.20.5").is_ge() {
        pairs.push((Loader::Fabric, Loader::OptiFine));
    }
    if mcver::compare(mc, "1.20.2").is_ge() {
        pairs.push((Loader::NeoForge, Loader::OptiFine));
    }
    if mcver::compare(mc, "1.13").is_ge() && mcver::compare(mc, "1.14.3").is_le() {
        pairs.push((Loader::Forge, Loader::OptiFine));
    }

    pairs
}
