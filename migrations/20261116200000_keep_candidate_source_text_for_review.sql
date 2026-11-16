-- Une alerte « source modifiée » sans extrait oblige à rouvrir la source pour
-- savoir si le changement compte : personne ne trie plus la file de revue.
-- La table garde donc le texte normalisé de la dernière lecture (fragments
-- visibles triés, liste des PDF liés, ou texte d'un PDF), pour calculer ce qui
-- a été ajouté ou retiré. Ce texte sert à la revue humaine (README.md §8.2) ;
-- il n'est jamais publié sur le site.
ALTER TABLE candidate_program_source_observations
    ADD COLUMN content_text TEXT,
    -- Fragments ajoutés et retirés lors du dernier changement, un par ligne,
    -- tronqués ; NULL quand la lecture précédente n'avait pas de texte gardé.
    ADD COLUMN last_added_text TEXT,
    ADD COLUMN last_removed_text TEXT,
    -- Nombre total de fragments ajoutés et retirés, avant troncature.
    ADD COLUMN last_added_count INTEGER CHECK (last_added_count IS NULL OR last_added_count >= 0),
    ADD COLUMN last_removed_count INTEGER CHECK (last_removed_count IS NULL OR last_removed_count >= 0);
