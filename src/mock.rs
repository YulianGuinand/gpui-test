//! Deterministic mock data: conversations and messages for the chat demo.
//!
//! Everything is generated from a seeded PRNG so the same scale always
//! produces the same dataset — performance runs stay comparable.
//! No external dependency: `std` only.

use std::fmt;
use std::sync::Arc;

// ── Seeded PRNG (xorshift64*) ───────────────────────────────────────────────

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform value in `0..n` (n > 0).
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }

    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            lo
        } else {
            lo + self.below(hi - lo + 1)
        }
    }

    pub fn chance(&mut self, pct: usize) -> bool {
        self.below(100) < pct
    }

    pub fn pick<'a>(&mut self, items: &'a [&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

// ── Scale ───────────────────────────────────────────────────────────────────

/// Dataset size selector — the UI can switch it to observe scaling.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scale {
    /// ~10 000 messages.
    Small,
    /// ~50 000 messages.
    Large,
    /// ~100 000 messages.
    Huge,
}

impl Scale {
    pub const ALL: [Scale; 3] = [Scale::Small, Scale::Large, Scale::Huge];

    pub fn total(self) -> usize {
        match self {
            Scale::Small => 10_000,
            Scale::Large => 50_000,
            Scale::Huge => 100_000,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Scale::Small => "10k",
            Scale::Large => "50k",
            Scale::Huge => "100k",
        }
    }

    pub fn next(self) -> Scale {
        match self {
            Scale::Small => Scale::Large,
            Scale::Large => Scale::Huge,
            Scale::Huge => Scale::Small,
        }
    }
}

impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ── Model ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Delivery {
    Sent,
    Delivered,
    Read,
}

impl Delivery {
    pub fn label(self) -> &'static str {
        match self {
            Delivery::Sent => "Sent",
            Delivery::Delivered => "Delivered",
            Delivery::Read => "Read",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    pub id: u64,
    /// Index into `Conversation::senders`.
    pub sender: usize,
    pub body: String,
    /// Unix seconds.
    pub at: u64,
    /// Only meaningful for outgoing messages.
    pub delivery: Option<Delivery>,
}

impl Message {
    pub fn is_outgoing(&self) -> bool {
        self.sender == 0
    }
}

#[derive(Clone, Debug)]
pub struct Conversation {
    pub id: u64,
    pub title: String,
    pub initials: String,
    /// `senders[0]` is always "You" (outgoing messages).
    pub senders: Vec<String>,
    pub unread: usize,
    pub messages: Arc<Vec<Message>>,
    /// `messages.len()` at generation time (kept after the user appends).
    pub planned: usize,
}

impl Conversation {
    pub fn preview(&self) -> &str {
        let last = self.messages.last();
        last.map(|m| m.body.as_str()).unwrap_or("")
    }

    pub fn last_at(&self) -> u64 {
        self.messages.last().map(|m| m.at).unwrap_or(0)
    }

    pub fn initials_for(&self, sender: usize) -> String {
        initials_of(self.senders.get(sender).map(String::as_str).unwrap_or("?"))
    }
}

// ── Word banks ──────────────────────────────────────────────────────────────

const NAMES: &[&str] = &[
    "Alice Moreau", "Bruno Lefèvre", "Camille Girard", "David Nakamura", "Élise Bernard",
    "Farid Haddad", "Grace Kim", "Hugo Martínez", "Inès Roux", "Jonas Weber",
    "Karim Benali", "Léa Fontaine", "Mateo Silva", "Nora Petit", "Omar Diallo",
    "Pauline Chevalier", "Quentin Roy", "Rina Tanaka", "Sofia Marino", "Thomas Vidal",
];

const GROUP_NAMES: &[&str] = &[
    "Équipe mobile", "Projet Aurora", "Stand-up produit", "Design sync", "Rust FR",
    "Family", "GPUI testers", "Release 2.4", "Bug triage", "Communauté Zed",
];

const SUBJECTS: &[&str] = &[
    "la release", "le build", "l'émulateur", "le review", "le ticket #482", "le design",
    "la doc", "le benchmark", "l'API", "le dashboard", "la migration", "le refactoring",
    "le pipeline CI", "la recette", "le sprint", "le backlog", "le patch", "le pipeline",
    "la charte", "le prototype", "le formulaire", "le cache", "l'index", "le viewport",
];

const VERBS: &[&str] = &[
    "est prêt", "bloque encore", "a été mergé", "prend du retard", "passe en revue",
    "fonctionne enfin", "casse sur Android", "est déployé", "demande une validation",
    "a besoin d'un dernier passage", "déborde", "gagne 20 %", "perd en netteté", "mérite une itération",
    "revient en prod ce soir", "attend ton retour", "a été corrigé", "repart de zéro",
];

const WORDS: &[&str] = &[
    "interface", "message", "conversation", "fenêtre", "rendu", "masque", "défilement",
    "potentiel", "budget", "latence", "widget", "buffer", "surface", "contexte", "événement",
    "touche", "écran", "état", "flux", "réponse", "historique", "brouillon", "jeton",
    "mesure", "profil", "tracé", "ombre", "grille", "police", "couleur", "icône", "cadre",
    "capteur", "réseau", "paquet", "dépendance", "compilation", "cible", "émulateur",
    "stabilité", "régression", "jalon", "objectif", "priorité", "étape", "retour", "note",
];

const CONNECTORS: &[&str] = &[
    "et", "mais", "donc", "puisque", "ensuite", "sinon", "pendant que", "tant que",
    "ce qui fait que", "d'ailleurs", "par ailleurs", "en attendant",
];

const SNIPPETS: &[&str] = &[
    "cargo ndk -t x86_64 -o jniLibs --platform 26 build",
    "adb shell am start -n com.example.my_gpui_mobile/dev.gpui.mobile.GpuiActivity",
    "MessageScroller::new(id, state, render_row)",
    "fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement",
    "gpui_kit::component::Theme::change(ThemeMode::Dark, None, cx)",
];

const LONG_FILLER: &[&str] = &[
    "Le point important est que la liste virtualisée ne remesure que la rangée qui change, donc le positionnement de lecture reste stable même quand le contenu grossit.",
    "J'ai relancé le build en release sur l'émulateur et les temps de frame redescendent, il faut confirmer sur un vrai téléphone avant de conclure.",
    "On gardera le composer compact au repos et on laissera le clavier natif piloter la zone de saisie, comme recommandé par la doc mobile.",
    "Si tu scrolle vers le haut pendant un envoi, le suivi de queue se détache et un bouton permet de revenir au dernier message.",
];

// ── Text generation ─────────────────────────────────────────────────────────

fn initials_of(name: &str) -> String {
    let mut out = String::new();
    for word in name.split_whitespace().take(2) {
        if let Some(c) = word.chars().next() {
            out.extend(c.to_uppercase());
        }
    }
    if out.is_empty() {
        out.push('?');
    }
    out
}

fn make_sentence(rng: &mut Rng) -> String {
    // ~15% of messages are long multi-clause ones, occasionally multi-line.
    let clauses = if rng.chance(15) { rng.range(4, 7) } else { rng.range(1, 3) };
    let mut s = String::with_capacity(160);

    s.push_str(&capitalize(rng.pick(WORDS).trim()));
    for _ in 0..clauses {
        s.push(' ');
        if rng.chance(20) {
            s.push_str(rng.pick(CONNECTORS));
            s.push(' ');
        }
        if rng.chance(12) {
            s.push_str(rng.pick(VERBS));
        } else {
            s.push_str(rng.pick(WORDS).trim());
            s.push(' ');
            s.push_str(rng.pick(CONNECTORS));
            s.push(' ');
            s.push_str(rng.pick(SUBJECTS));
        }
        s.push('.');
    }

    if rng.chance(8) {
        s.push_str("\n\n");
        s.push_str(rng.pick(SNIPPETS));
    } else if rng.chance(10) {
        s.push(' ');
        s.push_str(rng.pick(LONG_FILLER));
    }
    s
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn make_body(rng: &mut Rng) -> String {
    if rng.chance(4) {
        rng.pick(SNIPPETS).to_string()
    } else {
        make_sentence(rng)
    }
}

/// Simulated incoming reply, streamed word by word by the chat screen.
pub fn auto_reply(seed: u64) -> String {
    let mut rng = Rng::new(seed.wrapping_mul(2_654_435_761) | 1);
    let mut out = String::new();
    for _ in 0..rng.range(2, 3) {
        out.push_str(&make_sentence(&mut rng));
        out.push(' ');
    }
    out.push_str(rng.pick(LONG_FILLER));
    out.trim().to_string()
}

/// Current unix time in seconds.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1_760_000_000)
}

// ── Dates (std only) ────────────────────────────────────────────────────────

const DAY: u64 = 86_400;
const HOUR: u64 = 3_600;
const MINUTE: u64 = 60;

const MONTHS: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.",
    "nov.", "déc.",
];

const WDAYS: [&str; 7] = ["dim", "lun", "mar", "mer", "jeu", "ven", "sam"];

/// Civil-from-days (Howard Hinnant's algorithm) — enough for demo formatting.
fn civil_from_days(z: i64) -> (i64, u32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d, (yoe % 400) as u32)
}

fn parts(at: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (at / DAY) as i64;
    let secs = at % DAY;
    let (y, mo, d, _) = civil_from_days(days);
    (y, mo, d, (secs / HOUR) as u32, ((secs % HOUR) / MINUTE) as u32, (secs % MINUTE) as u32)
}

pub fn format_time(at: u64) -> String {
    let (_, _, _, h, m, _) = parts(at);
    format!("{h:02}:{m:02}")
}

pub fn format_day(at: u64, now: u64) -> String {
    let days_now = (now / DAY) as i64;
    let days_then = (at / DAY) as i64;
    let (_, mo, d, _, _, _) = parts(at);
    match days_now - days_then {
        0 => "Aujourd'hui".to_string(),
        1 => "Hier".to_string(),
        diff if diff < 7 => {
            let wd = civil_from_days(days_then);
            // weekday: 1970-01-01 was a Thursday (index 4)
            let idx = ((days_then + 4).rem_euclid(7)) as usize;
            let _ = wd;
            format!("{} {}", WDAYS[idx], d)
        }
        _ => format!("{d} {}", MONTHS[(mo - 1) as usize]),
    }
}

/// Compact list stamp: time for today, weekday for this week, date otherwise.
pub fn format_list_stamp(at: u64, now: u64) -> String {
    let days_now = (now / DAY) as i64;
    let days_then = (at / DAY) as i64;
    match days_now - days_then {
        0 => format_time(at),
        diff if diff < 7 => {
            let idx = ((days_then + 4).rem_euclid(7)) as usize;
            WDAYS[idx].to_string()
        }
        _ => {
            let (_, mo, d, _, _, _) = parts(at);
            format!("{d} {}", MONTHS[(mo - 1) as usize])
        }
    }
}

pub fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

// ── Database ────────────────────────────────────────────────────────────────

pub struct Database {
    pub scale: Scale,
    pub conversations: Vec<Conversation>,
    pub generation_ms: u128,
    /// Messages generated in the last `generate` call (== scale target).
    pub message_count: usize,
    /// "Now" used for this generation (unix seconds).
    pub now: u64,
}

impl Database {
    pub fn generate(scale: Scale) -> Database {
        let start = std::time::Instant::now();
        let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15 ^ scale.total() as u64);

        const CONVERSATIONS: usize = 50;
        let total = scale.total();
        // One deliberately huge thread (40% of the dataset) for stress tests.
        let stress = (total * 40) / 100;
        let rest = total - stress;
        let per = rest / (CONVERSATIONS - 1);

        // "Now": a fixed instant so formatting stays deterministic per run.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(1_760_000_000);

        let mut conversations = Vec::with_capacity(CONVERSATIONS);
        let mut message_count = 0usize;
        let mut id = 1u64;

        for index in 0..CONVERSATIONS {
            let is_group = index % 5 == 3;
            let is_stress = index == 0;

            let (title, senders) = if is_group {
                let title = GROUP_NAMES[index % GROUP_NAMES.len()].to_string();
                let mut senders = vec!["Vous".to_string()];
                let nb = rng.range(3, 5);
                for _ in 0..nb {
                    let name = NAMES[rng.below(NAMES.len())];
                    if !senders.iter().any(|s| s == name) {
                        senders.push(name.to_string());
                    }
                }
                (title, senders)
            } else {
                let name = NAMES[(index * 7 + 3) % NAMES.len()];
                (name.to_string(), vec!["Vous".to_string(), name.to_string()])
            };

            let count = if is_stress { stress } else { per + rng.below(per / 4 + 1) };
            message_count += count;

            // The thread spans up to 90 days, newest messages last.
            let span_days = if is_stress { 90 } else { rng.range(1, 60) };
            let start_at = now - (span_days as u64) * DAY - rng.below((HOUR * 6) as usize) as u64;
            let step = (((span_days as u64) * DAY) / count.max(1) as u64).max(20);

            let mut messages = Vec::with_capacity(count);
            let mut at = start_at;
            let outgoing_base = if is_stress { 55 } else { 45 };
            for i in 0..count {
                at += (step as f64 * (0.5 + rng.below(100) as f64 / 100.0)) as u64;
                if at > now {
                    at = now - rng.below(600) as u64;
                }
                let sender = if senders.len() > 1 && !rng.chance(outgoing_base) {
                    1 + rng.below(senders.len() - 1)
                } else if rng.chance(outgoing_base) {
                    0
                } else {
                    1
                };
                let delivery = if sender == 0 {
                    Some(if i + 1 == count {
                        Delivery::Sent
                    } else if rng.chance(70) {
                        Delivery::Read
                    } else {
                        Delivery::Delivered
                    })
                } else {
                    None
                };
                messages.push(Message {
                    id,
                    sender,
                    body: make_body(&mut rng),
                    at,
                    delivery,
                });
                id += 1;
            }

            let unread = if index % 3 == 0 { rng.range(0, 14) } else { 0 };
            let initials = initials_of(&title);
            conversations.push(Conversation {
                id: index as u64 + 1,
                title,
                initials,
                senders,
                unread: unread.min(messages.len()),
                planned: messages.len(),
                messages: Arc::new(messages),
            });
        }

        // Newest conversations first in the list.
        conversations.sort_by(|a, b| b.last_at().cmp(&a.last_at()));

        let generation_ms = start.elapsed().as_millis();
        Database {
            scale,
            conversations,
            generation_ms,
            message_count,
            now,
        }
    }

    pub fn total_messages(&self) -> usize {
        self.conversations.iter().map(|c| c.messages.len()).sum()
    }

    pub fn total_unread(&self) -> usize {
        self.conversations.iter().map(|c| c.unread).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let a = Database::generate(Scale::Small);
        let b = Database::generate(Scale::Small);
        assert_eq!(a.message_count, b.message_count);
        assert_eq!(
            a.conversations[0].messages[0].body,
            b.conversations[0].messages[0].body
        );
    }

    #[test]
    fn hits_target() {
        let db = Database::generate(Scale::Small);
        assert!(db.message_count >= Scale::Small.total());
        assert!(db.conversations.len() == 50);
    }
}
