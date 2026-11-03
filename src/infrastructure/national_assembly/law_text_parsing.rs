//! Découpage du HTML d'un texte de loi en articles (SPEC-en-bref RM-03, RM-04).
//!
//! Les textes de l'Assemblée n'ont pas de format commun : les propositions et
//! les textes adoptés portent leurs en-têtes dans des `p.assnat9ArticleNum`, le
//! projet de loi de finances dans des `td`/`h6` « ARTICLE 8 : intitulé », avec
//! un exposé des motifs par article et des numéros d'alinéa en blocs « (1) ».
//! Le texte est donc lu bloc par bloc (`p`, `td`, `h1`–`h6`, `div` sans bloc
//! enfant), le texte de chaque bloc décodé par `normalize_source_text`, sans
//! analyseur HTML : les balises en ligne (`span`, `a`, `sup`) ne coupent pas un
//! mot.

use sha2::{Digest, Sha256};

use crate::domain::law_text::{
    article_label, normalized_content, ArticleFingerprint, NewVersionArticle, VersionArticle,
};

use super::html_text::normalize_source_text;

/// Classe des en-têtes d'article des propositions et des textes adoptés. Quand
/// le document en porte, seuls ces blocs sont des en-têtes.
const ARTICLE_HEADING_CLASS: &str = "assnat9ArticleNum";

/// Fragments de classe des blocs qui ne sont pas le dispositif : exposé des
/// motifs, sommaire, notes de bas de page, commentaires (RM-04).
const EXCLUDED_CLASS_PARTS: [&str; 4] = ["expo", "TOC", "Endnote", "Comment"];

/// Début, en minuscules, du bloc qui clôt un texte adopté : « Délibéré en
/// séance publique, à Paris, le … », suivi de la signature.
const END_OF_TEXT_MARKER: &str = "délibéré en séance";

/// Éléments HTML lus comme des blocs de texte.
const BLOCK_TAGS: [&str; 9] = ["p", "td", "h1", "h2", "h3", "h4", "h5", "h6", "div"];

/// Éléments dont le contenu n'est pas du texte.
const RAW_TEXT_TAGS: [&str; 2] = ["style", "script"];

/// Ponctuation qui peut suivre un mot de division sans en faire partie.
const TRAILING_PUNCTUATION: [char; 4] = [':', '.', ',', ';'];

/// Une division tient sur une ligne courte ; au-delà, c'est une phrase.
const MAX_DIVISION_LINE_CHARS: usize = 200;

/// Chiffres d'un numéro d'alinéa « (n) » seul dans son bloc.
const MAX_PARAGRAPH_NUMBER_DIGITS: usize = 3;

const FINGERPRINT_PREFIX: &str = "sha256:";
const DIVISION_PATH_SEPARATOR: &str = " > ";
const DIVISION_TITLE_SEPARATOR: &str = " — ";

/// Les articles du dispositif d'un texte, dans l'ordre, chacun rangé sous ses
/// divisions et muni de l'empreinte de son contenu normalisé (RM-03, RM-04).
///
/// Ne sont pas des articles : ce qui précède le premier en-tête, l'exposé des
/// motifs, le sommaire, les notes, les numéros d'alinéa, les intitulés rédigés
/// par les auteurs, les titres qui ne sont ni un article ni une division
/// (annexes), la signature de fin de texte. Un en-tête sans contenu est écarté.
/// Le rang part de 1.
pub fn split_articles(html: &str) -> Vec<VersionArticle> {
    let blocks = read_blocks(html);
    let headings_marked = blocks
        .iter()
        .any(|block| block.class.contains(ARTICLE_HEADING_CLASS));
    let mut splitter = Splitter::default();
    for block in &blocks {
        splitter.read(block, headings_marked);
    }
    splitter.finish()
}

/// Article en cours de lecture.
struct OpenArticle {
    label: String,
    division: Option<String>,
    lines: Vec<String>,
}

#[derive(Default)]
struct Splitter {
    articles: Vec<VersionArticle>,
    open: Option<OpenArticle>,
    /// Divisions en vigueur, de la plus large à la plus fine : (niveau, texte).
    divisions: Vec<(u8, String)>,
    /// Le bloc précédent était une division : son intitulé peut suivre.
    after_division: bool,
}

impl Splitter {
    fn read(&mut self, block: &Block, headings_marked: bool) {
        if EXCLUDED_CLASS_PARTS
            .iter()
            .any(|part| block.class.contains(part))
            || is_paragraph_number(&block.text)
        {
            return;
        }

        let may_be_heading = !headings_marked || block.class.contains(ARTICLE_HEADING_CLASS);
        if may_be_heading {
            match article_label(&block.text) {
                Some(label) => {
                    self.start_article(label);
                    self.after_division = false;
                    return;
                }
                // Un en-tête que l'on ne sait pas lire n'ouvre rien, mais ne
                // laisse pas non plus l'article précédent s'étendre sur le sien.
                None if headings_marked => {
                    self.close_article();
                    self.after_division = false;
                    return;
                }
                None => {}
            }
        }

        if let Some(level) = division_level(&block.text) {
            self.close_article();
            self.divisions.retain(|(held, _)| *held < level);
            self.divisions.push((level, one_line(&block.text)));
            self.after_division = true;
            return;
        }
        if self.after_division && block.class.to_lowercase().contains("intit") {
            if let Some((_, title)) = self.divisions.last_mut() {
                title.push_str(DIVISION_TITLE_SEPARATOR);
                title.push_str(&one_line(&block.text));
            }
            self.after_division = false;
            return;
        }
        self.after_division = false;

        if block.is_heading() {
            // Titre ni article ni division (« I – Impôts », « États annexés ») :
            // il clôt l'article, et un titre de premier niveau le contexte.
            self.close_article();
            if block.tag == "h1" {
                self.divisions.clear();
            }
            return;
        }
        if block.text.to_lowercase().starts_with(END_OF_TEXT_MARKER) {
            self.close_article();
            return;
        }
        if let Some(open) = &mut self.open {
            open.lines.push(block.text.clone());
        }
    }

    fn start_article(&mut self, label: String) {
        self.close_article();
        let division = (!self.divisions.is_empty()).then(|| {
            self.divisions
                .iter()
                .map(|(_, title)| title.as_str())
                .collect::<Vec<_>>()
                .join(DIVISION_PATH_SEPARATOR)
        });
        self.open = Some(OpenArticle {
            label,
            division,
            lines: Vec::new(),
        });
    }

    fn close_article(&mut self) {
        let Some(open) = self.open.take() else {
            return;
        };
        if open.lines.is_empty() {
            return;
        }
        let content = open.lines.join("\n");
        let rank = u16::try_from(self.articles.len() + 1).unwrap_or(u16::MAX);
        self.articles.push(VersionArticle::new(NewVersionArticle {
            label: open.label,
            division: open.division,
            rank,
            fingerprint: fingerprint(&content),
            content,
        }));
    }

    fn finish(mut self) -> Vec<VersionArticle> {
        self.close_article();
        self.articles
    }
}

/// RM-03 : `sha256:<hex>` du contenu normalisé.
fn fingerprint(content: &str) -> ArticleFingerprint {
    let digest = Sha256::digest(normalized_content(content).as_bytes());
    ArticleFingerprint::new(format!("{FINGERPRINT_PREFIX}{digest:x}"))
}

/// Un bloc « (n) » seul est un numéro d'alinéa du projet de loi de finances.
fn is_paragraph_number(text: &str) -> bool {
    text.trim()
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .is_some_and(|digits| {
            (1..=MAX_PARAGRAPH_NUMBER_DIGITS).contains(&digits.len())
                && digits.chars().all(|c| c.is_ascii_digit())
        })
}

/// Niveau d'une division (0 partie, 1 titre, 2 chapitre, 3 section, 4
/// sous-section), si le bloc en est une. Une division commence par son mot et un
/// numéro ; un bloc cité entre guillemets (« Section 2 bis ») est du contenu.
fn division_level(text: &str) -> Option<u8> {
    let line = text.lines().next()?;
    if line.chars().count() > MAX_DIVISION_LINE_CHARS {
        return None;
    }
    let mut words = line.split_whitespace();
    let keyword = words
        .next()?
        .replace(['\u{2010}', '\u{2011}'], "-")
        .to_lowercase();
    let second = words.next();
    let numbered = || second.is_some_and(is_numeral);
    match keyword.trim_end_matches(TRAILING_PUNCTUATION) {
        "première" | "seconde" | "deuxième" | "troisième" => second
            .is_some_and(|word| {
                word.trim_end_matches(TRAILING_PUNCTUATION)
                    .eq_ignore_ascii_case("partie")
            })
            .then_some(0),
        "titre" => numbered().then_some(1),
        "chapitre" => numbered().then_some(2),
        "section" => numbered().then_some(3),
        "sous-section" => numbered().then_some(4),
        _ => None,
    }
}

/// Numéro de division : chiffres, chiffres romains, « premier », « Ier ».
fn is_numeral(word: &str) -> bool {
    let word = word.trim_end_matches(TRAILING_PUNCTUATION).to_lowercase();
    !word.is_empty()
        && (matches!(
            word.as_str(),
            "premier" | "ier" | "1er" | "unique" | "préliminaire" | "liminaire"
        ) || word.chars().all(|c| c.is_ascii_digit())
            || (word.len() <= 8 && word.chars().all(|c| "ivxlcdm".contains(c))))
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Bloc de texte lu dans le HTML.
struct Block {
    tag: &'static str,
    class: String,
    /// Texte décodé, une ligne par `<br>`, sans ligne vide.
    text: String,
}

impl Block {
    /// `h1` à `h6`.
    fn is_heading(&self) -> bool {
        self.tag.len() == 2 && self.tag.starts_with('h')
    }
}

/// Bloc ouvert, en attente de sa fermeture.
struct Frame {
    tag: &'static str,
    class: String,
    text: String,
    has_block_child: bool,
}

/// Lit les blocs feuilles du document, dans l'ordre.
///
/// Un bloc qui contient un autre bloc (`td` autour d'un `h6`, `div` autour d'un
/// `p`) n'est pas lu pour lui-même. Les commentaires, `style` et `script` sont
/// ignorés. Un `p` ouvert dans un `p` ferme le premier ; ce qui reste ouvert en
/// fin de document est fermé.
fn read_blocks(html: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut open: Vec<Frame> = Vec::new();
    let mut cursor = 0;

    while cursor < html.len() {
        let Some(offset) = html[cursor..].find('<') else {
            append_text(&mut open, &html[cursor..]);
            break;
        };
        let start = cursor + offset;
        append_text(&mut open, &html[cursor..start]);
        let from_tag = &html[start..];

        if from_tag.starts_with("<!--") {
            cursor = from_tag
                .find("-->")
                .map_or(html.len(), |end| start + end + "-->".len());
            continue;
        }
        if !starts_tag(from_tag) {
            append_text(&mut open, "<");
            cursor = start + 1;
            continue;
        }
        let Some(end) = tag_end(from_tag) else {
            break;
        };
        let tag = &from_tag[1..end];
        cursor = start + end + 1;

        let closing = tag.starts_with('/');
        let self_closing = tag.ends_with('/');
        let body = tag.trim_start_matches('/');
        let name_length = body
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(body.len());
        let name = body[..name_length].to_ascii_lowercase();

        if name == "br" {
            if !closing {
                append_raw_line_break(&mut open);
            }
        } else if RAW_TEXT_TAGS.contains(&name.as_str()) {
            if !closing && !self_closing {
                cursor = find_closing_tag(html, cursor, &name).unwrap_or(html.len());
            }
        } else if let Some(block_tag) = BLOCK_TAGS
            .iter()
            .copied()
            .find(|candidate| *candidate == name)
        {
            if closing {
                close_block(&mut open, &mut blocks, block_tag);
            } else if !self_closing {
                if block_tag == "p" {
                    while open.last().is_some_and(|frame| frame.tag == "p") {
                        close_top(&mut open, &mut blocks);
                    }
                }
                if let Some(parent) = open.last_mut() {
                    parent.has_block_child = true;
                }
                open.push(Frame {
                    tag: block_tag,
                    class: class_of(tag),
                    text: String::new(),
                    has_block_child: false,
                });
            }
        }
    }

    while close_top(&mut open, &mut blocks).is_some() {}
    blocks
}

/// Le `<` ouvre une balise, un commentaire ou une déclaration (et non un
/// « inférieur à » dans le texte).
fn starts_tag(from_angle: &str) -> bool {
    from_angle[1..]
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '/' | '!' | '?'))
}

/// Position du `>` qui ferme la balise, hors des guillemets des attributs.
fn tag_end(from_angle: &str) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (index, byte) in from_angle.bytes().enumerate().skip(1) {
        match (quote, byte) {
            (Some(open), found) if found == open => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(byte),
            (None, b'>') => return Some(index),
            (None, _) => {}
        }
    }
    from_angle.find('>')
}

/// Position, à partir de `from`, de la balise fermante `</name`.
fn find_closing_tag(html: &str, from: usize, name: &str) -> Option<usize> {
    html[from..]
        .match_indices("</")
        .find(|(index, _)| {
            html[from + index + 2..]
                .get(..name.len())
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
        })
        .map(|(index, _)| from + index)
}

/// Valeur de l'attribut `class` d'une balise, vide si elle n'en a pas.
fn class_of(tag: &str) -> String {
    let lowered = tag.to_ascii_lowercase();
    let Some(position) = lowered
        .match_indices("class=")
        .map(|(index, _)| index)
        .find(|index| {
            lowered[..*index]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace)
        })
    else {
        return String::new();
    };
    let value = &tag[position + "class=".len()..];
    match value.chars().next() {
        Some(quote @ ('"' | '\'')) => value[1..]
            .split(quote)
            .next()
            .unwrap_or_default()
            .to_string(),
        _ => value
            .split(|c: char| c.is_whitespace() || c == '>')
            .next()
            .unwrap_or_default()
            .to_string(),
    }
}

/// Texte du source, dont les retours à la ligne ne sont que de la mise en page.
fn append_text(open: &mut [Frame], text: &str) {
    if let Some(frame) = open.last_mut() {
        frame.text.extend(text.chars().map(|c| {
            if matches!(c, '\n' | '\r' | '\t') {
                ' '
            } else {
                c
            }
        }));
    }
}

fn append_raw_line_break(open: &mut [Frame]) {
    if let Some(frame) = open.last_mut() {
        frame.text.push('\n');
    }
}

/// Ferme le bloc `tag` et ceux restés ouverts au-dessus de lui.
fn close_block(open: &mut Vec<Frame>, blocks: &mut Vec<Block>, tag: &str) {
    if !open.iter().any(|frame| frame.tag == tag) {
        return;
    }
    while let Some(closed) = close_top(open, blocks) {
        if closed == tag {
            break;
        }
    }
}

/// Ferme le bloc le plus récent et en garde le texte s'il est une feuille.
fn close_top(open: &mut Vec<Frame>, blocks: &mut Vec<Block>) -> Option<&'static str> {
    let frame = open.pop()?;
    let tag = frame.tag;
    if !frame.has_block_child {
        push_block(frame, blocks);
    }
    Some(tag)
}

fn push_block(frame: Frame, blocks: &mut Vec<Block>) {
    let text = normalize_source_text(&frame.text)
        .lines()
        .map(one_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if !text.is_empty() {
        blocks.push(Block {
            tag: frame.tag,
            class: frame.class,
            text,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extrait réduit de la proposition n° 2218 : un exposé des motifs en
    /// paragraphes sans classe, puis deux articles `assnat9ArticleNum`, les
    /// numéros d'alinéa de l'article 2 sont des images, une note de bas de page
    /// suit le dispositif.
    const BILL_2218: &str = r##"<!DOCTYPE html>
<html lang="fr-FR">
<head>
    <title>PIONANR5L17B2218</title>
    <style type="text/css">
        .assnat9ArticleNum { margin-top:24pt; text-align:center }
        .assnatEndnoteText { text-align:justify; font-size:10pt }
    </style>
    <script>if (1 < 2) { document.title = "<p class=\"assnat9ArticleNum\">Article 9</p>"; }</script>
</head>
<body>
<div class="assnatSection2">
  <p class="assnat4titreintit" style="margin-top:6pt"><span style="font-weight:normal">EXPOSÉ DES MOTIFS</span></p>
  <p style="margin-bottom:12pt">M<span style="font-variant:small-caps">esdames</span>, M<span style="font-variant:small-caps">essieurs</span>,</p>
  <p style="margin-bottom:12pt">Article 51 de la loi du 9 avril 2024 a introduit deux dérogations conséquentes aux règles de la prescription.</p>
  <p style="margin-bottom:12pt">La présente proposition vise donc à abroger ces dispositions.</p>
</div>
<div class="assnatSection3">
  <p class="assnat4titreintit" style="margin-top:12pt"><span>proposition de loi</span></p>
  <p class="assnat9ArticleNum">Article 1<span style="font-size:9.33pt; vertical-align:super">er</span> </p>
  <p class="assnatLoiTexte">Le III de l&#8217;article 51 de la loi n°&nbsp;2024‑322 du 9 avril 2024 est abrogée.</p>
  <p class="assnat9ArticleNum">
    Article 2
  </p>
  <p class="assnatLoiTexte"><span style="height:0pt; display:block; position:absolute"><img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAC0AAAAeCAYAAAC49JeZ"/></span>L’article 2272 du code civil s’applique pleinement dans les collectivités régies par l’article 73 de la Constitution.</p>
  <p class="assnatLoiTexte"><span style="height:0pt; display:block; position:absolute"><img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAC0AAAAeCAYAAAC49JeZ"/></span>L’article 2261 du code civil s’applique également sans dérogation.</p>
</div>
<hr style="width:33%">
<div id="_edn1">
  <p class="assnatEndnoteText"><a href="#_ednref1"><span class="assnatEndnoteReference">[(1)]</span></a> Ce groupe est composé de : Mme Nadège ABOMANGOLI, M. Laurent ALEXANDRE.</p>
</div>
</body>
</html>"##;

    /// Extrait réduit du texte adopté n° 323 : des chapitres avec leur intitulé,
    /// des divisions citées entre guillemets dans le contenu d'un article, un
    /// article supprimé, un « bis (nouveau) » et la signature de fin de texte.
    const ADOPTED_323: &str = r##"<html><head><title>PIONANR5L17BTA0323</title></head><body>
<div class="assnatSection2">
  <p class="assnat4titreNum">TITRE I<span style="font-size:9.33pt; vertical-align:super">er</span></p>
  <p class="assnat4titreintit">Fin de vie</p>
  <p class="assnat5ChapitreNum0" style="margin-top:18pt">Chapitre I<span style="font-size:9.33pt; vertical-align:super">er</span></p>
  <p class="assnat5ChapitreIntit">Définition</p>
  <p class="assnat9ArticleNum">Article 1<span style="font-size:9.33pt; vertical-align:super">er</span></p>
  <p class="assnatLoiTexte">Après le mot : « santé », la fin de l’intitulé du chapitre I<span style="font-size:9.33pt; vertical-align:super">er</span> du titre I<span style="font-size:9.33pt; vertical-align:super">er</span> est ainsi rédigée : « et de la fin de vie ».</p>
  <p class="assnat9ArticleNum">Article 2</p>
  <p class="assnatLoiTexte"><span style="letter-spacing:-0.2pt">Après la section</span> 2 du chapitre I<span style="font-size:9.33pt; vertical-align:super">er</span> est insérée une section 2 <span style="font-style:italic">bis</span> ainsi rédigée :</p>
  <p class="assnat6SectionNum0"><span style="height:0pt"><img src="data:image/png;base64,AAAA"/></span>« Section 2 <span style="font-style:normal">bis</span></p>
  <p class="assnat6SectionIntit0"><span style="height:0pt"><img src="data:image/png;base64,AAAA"/></span>« Droit à l’aide à mourir</p>
  <p class="assnat7Sous-sectionNum"><span style="height:0pt"><img src="data:image/png;base64,AAAA"/></span>« Sous‑section 1</p>
  <p class="assnatLoiTexte">« <span style="font-style:italic">Art. L. 1111‑12‑1</span>. – I. – Le droit à l’aide à mourir est le droit de recourir à une substance létale. »</p>
  <p class="assnat5ChapitreNum0">Chapitre II</p>
  <p class="assnat5ChapitreIntit">Conditions d’accès</p>
  <p class="assnat9ArticleNum">Article 4</p>
  <p class="assnatLoiTexte">Le texte de l’article 4.</p>
  <p class="assnat5ChapitreNum0">Chapitre VI</p>
  <p class="assnat5ChapitreIntit">Dispositions pénales</p>
  <p class="assnat9ArticleNum">Article 17</p>
  <p class="assnatMentionsousarticle">(Supprimé)</p>
  <p class="assnat9ArticleNum">Article 19 <span style="font-style:italic">bis</span> <span style="font-style:italic">(nouveau)</span></p>
  <p class="assnatLoiTexte">Un projet de loi de ratification est déposé devant le Parlement.</p>
  <p class="assnatLoiTexte" style="page-break-after:avoid"><a id="_Hlk230010939"> </a></p>
  <p style="text-align:right"><span style="font-style:italic">Délibéré en séance publique, à Paris, le 30 juin 2026.</span></p>
  <p style="text-align:center"><span>La Présidente,</span></p>
  <p style="text-align:center"><span>Signé : Yaël BRAUN-PIVET</span></p>
</div>
</body></html>"##;

    /// Extrait réduit du projet de loi de finances n° 3210 : sommaire, exposé
    /// général, en-têtes `td`/`h6` « ARTICLE n : intitulé », numéros d'alinéa
    /// `(n)`, exposé de chaque article, divisions en `h2`/`h3`, puis états
    /// annexés.
    const BUDGET_3210: &str = r##"<html><head><title>PRJLANR5L17B3210</title></head><body>
<h1 style="margin-top:12pt">Exposé général des motifs</h1>
<p class="assnatBodyText">Article 8 de la loi organique relative aux lois de finances : le Gouvernement présente ses orientations.</p>
<p class="assnatTOC6"><a href="#_Toc241710665"><span class="assnatHyperlink" style="font-weight:bold">ARTICLE</span><span class="assnatHyperlink"> </span><span class="assnatHyperlink" style="font-weight:bold">liminaire</span><span class="assnatHyperlink"> : Prévisions de solde structurel</span></a></p>
<h1 style="margin-top:12pt">Articles du projet de loi avec exposé des motifs</h1>
<table style="width:100%"><tr><td style="width:497.5pt; border-left:0.75pt solid #0070b9">
  <h6 style="margin:1pt 1pt 1pt 4pt"><a id="_Toc241710665"><span style="font-weight:bold">ARTICLE</span> <span style="font-weight:bold">liminaire</span> <span style="color:#ffffff">: : </span><br><span>Prévisions de solde structurel et de solde effectif de l’ensemble des administrations publiques</span></a></h6>
</td></tr></table>
<p class="assnatFPFdebutartexte"> </p>
<div style="width:28.35pt"><p class="assnatPastille"><span style="font-size:7pt">(1)</span></p></div>
<p class="assnatFPFprojetloiartexte">Les prévisions de solde structurel et de solde effectif sont présentées dans le tableau ci-après.</p>
<div style="width:28.35pt"><p class="assnatPastille" style="line-height:normal">(2)</p></div>
<table style="width:453.75pt"><tr>
  <td><p class="assnatligneTableauC">Solde structurel (1)</p></td>
  <td><p class="assnatligneTableauC" style="text-align:center">‑5,2</p></td>
</tr></table>
<p class="assnatFPFfinartexte"> </p>
<p class="assnatFPFexpogentitre1">Exposé des motifs de l’article liminaire</p>
<p class="assnatFPFexpogentexte">L’article liminaire présente les prévisions de solde pour l’année 2027.</p>
<h2>PREMIÈRE PARTIE : CONDITIONS GÉNÉRALES DE L’ÉQUILIBRE FINANCIER</h2>
<h3>TITRE PREMIER : DISPOSITIONS RELATIVES AUX RESSOURCES</h3>
<h4>I – Impôts et ressources autorisées</h4>
<h5>A – Autorisation de perception des impôts et produits</h5>
<table style="width:100%"><tr><td><h6>ARTICLE 1 : <br>Autorisation de percevoir les impôts existants</h6></td></tr></table>
<div style="width:28.35pt"><p class="assnatPastille">(1)</p></div>
<p class="assnatFPFprojetloiartexte">I. – La perception des ressources de l’État et des impositions de toute nature est autorisée.</p>
<p class="assnatFPFexpogentexte">Cet article autorise la perception des impôts existants.</p>
<h5>B – Mesures fiscales</h5>
<table style="width:100%"><tr><td><h6>ARTICLE 23 <br>Instauration d’un prélèvement exceptionnel</h6></td></tr></table>
<p class="assnatFPFprojetloiartexte">Un prélèvement exceptionnel est institué.</p>
<p class="assnatFPFexpogentexte">Le prélèvement exceptionnel porte sur les avoirs en déshérence.</p>
<h1 style="margin-top:12pt">États législatifs annexés</h1>
<h6>Etat A - Voies et moyens pour 2027 <br>(Article 57 du projet de loi)</h6>
<p class="assnatetatalrM">Impôt sur le revenu</p>
</body></html>"##;

    fn labels(articles: &[VersionArticle]) -> Vec<&str> {
        articles.iter().map(VersionArticle::label).collect()
    }

    fn bill_with(articles: &str) -> String {
        format!("<html><body>{articles}</body></html>")
    }

    fn article_html(heading: &str, text: &str) -> String {
        format!(
            "<p class=\"assnat9ArticleNum\">{heading}</p><p class=\"assnatLoiTexte\">{text}</p>"
        )
    }

    #[test]
    fn should_split_dispositif_into_articles_when_bill_uses_article_heading_class() {
        let articles = split_articles(BILL_2218);

        assert_eq!(labels(&articles), vec!["1er", "2"]);
        assert_eq!(
            articles[0].content(),
            "Le III de l’article 51 de la loi n° 2024‑322 du 9 avril 2024 est abrogée."
        );
        assert_eq!(
            articles[1].content(),
            "L’article 2272 du code civil s’applique pleinement dans les collectivités régies par l’article 73 de la Constitution.\nL’article 2261 du code civil s’applique également sans dérogation."
        );
        assert_eq!(articles[0].rank(), 1);
        assert_eq!(articles[1].rank(), 2);
        assert_eq!(articles[0].division(), None);
    }

    #[test]
    fn should_ignore_explanatory_statement_when_it_precedes_the_dispositif() {
        let articles = split_articles(BILL_2218);

        for article in &articles {
            assert!(!article.content().contains("Mesdames"));
            assert!(!article.content().contains("dérogations conséquentes"));
            assert!(!article.content().contains("La présente proposition"));
            assert!(!article.content().contains("ABOMANGOLI"));
        }
        assert!(
            !labels(&articles).contains(&"51"),
            "a sentence of the statement that starts with « Article 51 » is not a heading"
        );
    }

    #[test]
    fn should_file_articles_under_their_division_when_text_has_titles_and_chapters() {
        let articles = split_articles(ADOPTED_323);

        assert_eq!(labels(&articles), vec!["1er", "2", "4", "17", "19 bis"]);
        assert_eq!(
            articles[0].division(),
            Some("TITRE Ier — Fin de vie > Chapitre Ier — Définition")
        );
        assert_eq!(articles[1].division(), articles[0].division());
        assert_eq!(
            articles[2].division(),
            Some("TITRE Ier — Fin de vie > Chapitre II — Conditions d’accès")
        );
        assert_eq!(
            articles[4].division(),
            Some("TITRE Ier — Fin de vie > Chapitre VI — Dispositions pénales")
        );
    }

    #[test]
    fn should_keep_quoted_divisions_in_the_content_when_an_article_inserts_them() {
        let articles = split_articles(ADOPTED_323);
        let content = articles[1].content();

        assert!(content.contains("« Section 2 bis"));
        assert!(content.contains("« Sous‑section 1"));
        assert!(content.contains("Art. L. 1111‑12‑1"));
        assert_eq!(articles[1].division(), articles[0].division());
    }

    #[test]
    fn should_keep_deleted_article_and_stop_before_signature_when_text_is_adopted() {
        let articles = split_articles(ADOPTED_323);

        assert_eq!(articles[3].label(), "17");
        assert_eq!(articles[3].content(), "(Supprimé)");
        assert_eq!(
            articles[4].content(),
            "Un projet de loi de ratification est déposé devant le Parlement."
        );
    }

    #[test]
    fn should_skip_article_explanations_and_paragraph_numbers_when_text_is_a_budget_bill() {
        let articles = split_articles(BUDGET_3210);

        assert_eq!(labels(&articles), vec!["liminaire", "1", "23"]);
        assert_eq!(
            articles[0].content(),
            "Les prévisions de solde structurel et de solde effectif sont présentées dans le tableau ci-après.\nSolde structurel (1)\n‑5,2"
        );
        assert_eq!(
            articles[1].content(),
            "I. – La perception des ressources de l’État et des impositions de toute nature est autorisée."
        );
        assert_eq!(
            articles[2].content(),
            "Un prélèvement exceptionnel est institué."
        );
        let division = Some(
            "PREMIÈRE PARTIE : CONDITIONS GÉNÉRALES DE L’ÉQUILIBRE FINANCIER > TITRE PREMIER : DISPOSITIONS RELATIVES AUX RESSOURCES",
        );
        assert_eq!(articles[0].division(), None);
        assert_eq!(articles[1].division(), division);
        assert_eq!(articles[2].division(), division);
    }

    #[test]
    fn should_leave_headings_of_the_summary_and_the_annexes_out_when_text_is_a_budget_bill() {
        let articles = split_articles(BUDGET_3210);
        let everything: String = articles.iter().map(VersionArticle::content).collect();

        assert!(!everything.contains("Impôt sur le revenu"));
        assert!(!everything.contains("Etat A"));
        assert!(!everything.contains("Instauration"));
        assert!(!everything.contains("Autorisation de percevoir"));
        assert!(!everything.contains("Cet article autorise"));
    }

    #[test]
    fn should_fingerprint_with_sha256_prefix_when_article_is_split() {
        let articles = split_articles(&bill_with(&article_html("Article 4", "Le texte.")));
        let fingerprint = articles[0].fingerprint().as_str();

        assert!(fingerprint.starts_with("sha256:"));
        let digest = &fingerprint["sha256:".len()..];
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn should_give_same_fingerprint_when_article_is_renumbered_or_only_markers_differ() {
        let before = split_articles(&bill_with(&article_html(
            "Article 4",
            "4° (nouveau) Le médecin évalue la demande.",
        )));
        let after = split_articles(&bill_with(&article_html(
            "Article 5",
            "4° Le médecin évalue la demande.",
        )));
        let changed = split_articles(&bill_with(&article_html(
            "Article 5",
            "4° Le médecin refuse la demande.",
        )));

        assert_eq!(before[0].fingerprint(), after[0].fingerprint());
        assert_ne!(before[0].fingerprint(), changed[0].fingerprint());
    }

    #[test]
    fn should_return_no_article_when_html_has_no_dispositif() {
        let statement_only = r##"<html><body>
            <p class="assnat4titreintit">EXPOSÉ DES MOTIFS</p>
            <p>Article 3 de la loi du 9 avril 2024 : un constat.</p>
            <p class="assnatTOC6">Article 1er : sommaire</p>
        </body></html>"##;

        assert!(split_articles(statement_only).is_empty());
        assert!(split_articles("").is_empty());
        assert!(split_articles("pas du HTML").is_empty());
    }

    #[test]
    fn should_drop_article_when_nothing_follows_its_heading() {
        let html = bill_with(&format!(
            "<p class=\"assnat9ArticleNum\">Article 1er</p>{}",
            article_html("Article 2", "Le texte.")
        ));

        assert_eq!(labels(&split_articles(&html)), vec!["2"]);
    }

    #[test]
    fn should_read_unclosed_paragraphs_and_line_breaks_when_markup_is_loose() {
        let html = "<html><body><p class=\"assnat9ArticleNum\">Article 1er\
            <p class=\"assnatLoiTexte\">Première ligne<br>seconde ligne</body></html>";

        let articles = split_articles(html);

        assert_eq!(labels(&articles), vec!["1er"]);
        assert_eq!(articles[0].content(), "Première ligne\nseconde ligne");
    }

    /// Lit le HTML réel désigné par `TEXT_VERSION_HTML` (par exemple
    /// `PIONANR5L17BTA0323.html`, téléchargé depuis
    /// `https://www.assemblee-nationale.fr/dyn/docs/<uid>.raw`).
    #[test]
    #[ignore = "reads a real text from the file named by TEXT_VERSION_HTML"]
    fn should_split_real_text_when_html_file_is_given() {
        let path = std::env::var("TEXT_VERSION_HTML").expect("TEXT_VERSION_HTML");
        let html = std::fs::read_to_string(&path).expect("readable html file");

        let articles = split_articles(&html);

        for article in &articles {
            println!(
                "{:>4} | {:<10} | {} | {} chars | {}",
                article.rank(),
                article.label(),
                article.division().unwrap_or("-"),
                article.content().chars().count(),
                article.content().lines().next().unwrap_or("")
            );
        }
        assert!(!articles.is_empty(), "no article recognised in {path}");
    }
}
