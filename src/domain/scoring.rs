use super::dossier::Score;

/// Score heuristique d'un dossier: trois sous-notes de 0 a 10 ramenees a un
/// total sur 100.
///
/// - avancement: lu dans le libelle de la derniere activite (promulgation = 10,
///   vote solennel = 9, ... depot = 2, defaut = 1);
/// - ampleur: lue dans les mots du titre (constitution ou finances = 10,
///   ratification ou accord = 2, defaut = 4);
/// - dynamique: croissante avec le nombre d'actes (0 ou 1 = 2, 10 et plus = 10).
///
/// Le total pondere avancement x3, ampleur x2, dynamique x1 sur un maximum de 60.
pub fn compute_score(title: &str, last_activity_label: &str, act_count: usize) -> Score {
    let progress = score_progress(last_activity_label);
    let magnitude = score_magnitude(title);
    let momentum = score_momentum(act_count);
    let total = ((progress as u16 * 3 + magnitude as u16 * 2 + momentum as u16) * 100 / 60) as u8;
    Score::new(progress, magnitude, momentum, total).expect("compute_score produces valid ranges")
}

fn score_progress(label: &str) -> u8 {
    let lower = label.to_lowercase();
    if lower.contains("promulgation") {
        10
    } else if lower.contains("vote solennel") || lower.contains("scrutin public") {
        9
    } else if lower.contains("adoption") || lower.contains("discussion en séance") {
        7
    } else if lower.contains("2e lecture") || lower.contains("2ème lecture") {
        6
    } else if lower.contains("1ère lecture") || lower.contains("1ere lecture") {
        5
    } else if lower.contains("commission") {
        4
    } else if lower.contains("renvoi") {
        3
    } else if lower.contains("dépôt") || lower.contains("depot") {
        2
    } else {
        1
    }
}

fn score_magnitude(title: &str) -> u8 {
    let lower = title.to_lowercase();

    if lower.contains("constitution") || lower.contains("constitutionnel") {
        10
    } else if lower.contains("finances")
        || lower.contains("financement")
        || lower.contains("budget")
    {
        10
    } else if lower.contains("sécurité")
        || lower.contains("securite")
        || lower.contains("défense")
        || lower.contains("defense")
    {
        8
    } else if lower.contains("santé")
        || lower.contains("sante")
        || lower.contains("retraite")
        || lower.contains("éducation")
        || lower.contains("education")
        || lower.contains("justice")
    {
        8
    } else if lower.contains("réforme") || lower.contains("reforme") {
        7
    } else if lower.contains("environnement")
        || lower.contains("climat")
        || lower.contains("énergie")
        || lower.contains("energie")
        || lower.contains("travail")
        || lower.contains("emploi")
        || lower.contains("logement")
        || lower.contains("immigration")
        || lower.contains("numérique")
        || lower.contains("numerique")
    {
        6
    } else if lower.contains("ratification")
        || lower.contains("convention")
        || lower.contains("accord")
        || lower.contains("approbation")
    {
        2
    } else {
        4
    }
}

fn score_momentum(act_count: usize) -> u8 {
    match act_count {
        0..=1 => 2,
        2..=3 => 4,
        4..=6 => 6,
        7..=9 => 8,
        _ => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promulgation_scores_highest_progress() {
        let score = compute_score("Texte lambda", "Promulgation de la loi", 1);
        assert_eq!(score.progress(), 10);
    }

    #[test]
    fn solemn_vote_label_gives_progress_9() {
        let score = compute_score("Texte", "Vote solennel", 1);
        assert_eq!(score.progress(), 9);
    }

    #[test]
    fn depot_scores_low() {
        let score = compute_score("Texte", "Dépôt", 1);
        assert_eq!(score.progress(), 2);
    }

    #[test]
    fn plf_scores_max_magnitude() {
        let score = compute_score("Projet de loi de finances pour 2026", "Dépôt", 1);
        assert_eq!(score.magnitude(), 10);
    }

    #[test]
    fn reforme_scores_medium_magnitude() {
        let score = compute_score("Réforme des retraites", "Dépôt", 1);
        assert_eq!(score.magnitude(), 8);
    }

    #[test]
    fn ordinary_text_scores_default_magnitude() {
        let score = compute_score("Texte divers", "Dépôt", 1);
        assert_eq!(score.magnitude(), 4);
    }

    #[test]
    fn ratification_scores_lowest_magnitude() {
        let score = compute_score("Ratification d'un accord", "Dépôt", 1);
        assert_eq!(score.magnitude(), 2);
    }

    #[test]
    fn constitution_scores_max_magnitude() {
        let score = compute_score("Révision constitutionnelle", "Dépôt", 1);
        assert_eq!(score.magnitude(), 10);
    }

    #[test]
    fn sante_scores_high_magnitude() {
        let score = compute_score("Accès aux soins de santé", "Dépôt", 1);
        assert_eq!(score.magnitude(), 8);
    }

    #[test]
    fn environnement_scores_medium_magnitude() {
        let score = compute_score("Protection de l'environnement", "Dépôt", 1);
        assert_eq!(score.magnitude(), 6);
    }

    #[test]
    fn many_acts_scores_high_momentum() {
        let score = compute_score("Texte", "Dépôt", 12);
        assert_eq!(score.momentum(), 10);
    }

    #[test]
    fn few_acts_scores_low_momentum() {
        let score = compute_score("Texte", "Dépôt", 1);
        assert_eq!(score.momentum(), 2);
    }

    #[test]
    fn moderate_acts_scores_mid_momentum() {
        let score = compute_score("Texte", "Dépôt", 5);
        assert_eq!(score.momentum(), 6);
    }

    #[test]
    fn total_is_normalized_on_100() {
        let score = compute_score("Projet de loi de finances", "Promulgation de la loi", 15);
        // (10*3 + 10*2 + 10*1) * 100 / 60 = 100
        assert_eq!(score.total(), 100);

        let score = compute_score("Ratification d'un accord", "Dépôt", 1);
        // (2*3 + 2*2 + 2*1) * 100 / 60 = 20
        assert_eq!(score.total(), 20);
    }

    #[test]
    fn second_reading_label_gives_progress_6() {
        let score = compute_score("Texte", "2e lecture", 1);
        assert_eq!(score.progress(), 6);
    }

    #[test]
    fn first_reading_label_gives_progress_5() {
        let score = compute_score("Texte", "1ère lecture", 1);
        assert_eq!(score.progress(), 5);
    }
}
