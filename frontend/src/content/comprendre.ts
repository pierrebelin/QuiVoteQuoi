/**
 * Contenu de la page « Comprendre », en questions.
 *
 * Les deux niveaux sont disjoints : `detaille` suppose acquises les réponses
 * de `debutant` et ne les répète pas. Une même question ne doit donc jamais
 * exister dans les deux — deux formulations d'un même fait divergeraient à la
 * première correction.
 *
 * Corollaire : chaque section doit porter au moins une question par niveau,
 * sinon elle disparaît pour la moitié des lecteurs.
 *
 * **Aucun chiffre ici.** Ni volumétrie, ni pourcentage, ni date de couverture :
 * ces valeurs bougent à chaque ingestion et une page d'explication figée les
 * transformerait en faux. Les chiffres vivent sur les pages qui les servent
 * depuis la base, avec leur source. Les références juridiques (articles 40 et
 * 45 de la Constitution) ne sont pas des chiffres : elles ne bougent pas.
 *
 * Les identifiants de question sont des ancres publiques : les pages de vote y
 * renvoient. Les renommer casse des liens.
 */

export type Niveau = 'debutant' | 'detaille'

export type Bloc =
  | { kind: 'p'; texte: string }
  | { kind: 'ul'; intro?: string; items: string[] }
  | { kind: 'source'; libelle: string; lien: string; precision?: string }

export type Question = {
  id: string
  niveau: Niveau
  question: string
  reponse: Bloc[]
}

export type Section = {
  id: string
  titre: string
  questions: Question[]
}

export const SECTIONS: Section[] = [
  {
    id: 'bases',
    titre: 'Les bases',
    questions: [
      {
        id: 'q-quoi',
        niveau: 'debutant',
        question: 'Que montre ce site ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Les votes de l'Assemblée nationale, tels que l'Assemblée les publie : qui a voté quoi, quel jour, sur quel objet.",
          },
          {
            kind: 'p',
            texte:
              "Chaque chiffre affiché vient du jeu de données officiel et renvoie à sa source. Le site n'évalue ni le contenu d'un texte, ni le comportement d'un groupe : il ne produit ni note, ni classement, ni commentaire.",
          },
        ],
      },
      {
        id: 'q-perimetre',
        niveau: 'debutant',
        question: 'Quelle assemblée est couverte ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "L'Assemblée nationale seule. Le Sénat n'est pas couvert : un texte voté à l'Assemblée peut y avoir un parcours que ce site n'affiche pas.",
          },
          {
            kind: 'p',
            texte:
              "La période couverte est indiquée sur les pages qui servent les données, pas ici : elle s'étend à chaque ingestion.",
          },
        ],
      },
      {
        id: 'q-mise-a-jour',
        niveau: 'detaille',
        question: 'À quelle fréquence les données sont-elles mises à jour ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Au fil de l'eau, depuis l'open data de l'Assemblée, sans validation éditoriale préalable. Une correction publiée par la source remplace la version précédente affichée ici.",
          },
        ],
      },
      {
        id: 'q-selection',
        niveau: 'detaille',
        question: 'Certains votes sont-ils écartés de l’affichage ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Non. Le site n'écarte aucun scrutin publié par la source. Les classements et filtres proposés ordonnent l'affichage ; ils ne retirent rien de la base.",
          },
          {
            kind: 'p',
            texte:
              "Sur un site de transparence, choisir ce qu'on montre est déjà un acte éditorial. La contrainte est donc d'exposer tout ce que la source publie, y compris ce qui n'a été rattaché à aucun sujet.",
          },
        ],
      },
    ],
  },
  {
    id: 'vocabulaire',
    titre: 'Le vocabulaire',
    questions: [
      {
        id: 'q-texte-loi',
        niveau: 'debutant',
        question: 'Projet de loi, proposition de loi : quelle différence ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Un projet de loi est déposé par le Gouvernement. Une proposition de loi est déposée par un ou plusieurs parlementaires. Le parcours d'examen est ensuite le même.",
          },
          {
            kind: 'p',
            texte:
              "L'Assemblée examine aussi des propositions de résolution, qui expriment une position sans créer de règle de droit, et des motions de censure, qui ne portent sur aucun texte.",
          },
        ],
      },
      {
        id: 'q-article',
        niveau: 'debutant',
        question: 'Qu’est-ce qu’un article ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Un texte de loi est découpé en articles numérotés. Chacun peut être discuté, modifié et mis aux voix séparément, avant un vote final sur l'ensemble du texte.",
          },
        ],
      },
      {
        id: 'q-amendement',
        niveau: 'debutant',
        question: 'Qu’est-ce qu’un amendement ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Une modification proposée à un texte en cours d'examen : ajouter, supprimer ou réécrire tout ou partie d'un article.",
          },
          {
            kind: 'p',
            texte:
              "Un amendement peut être déposé par un député, un groupe, la commission saisie du texte ou le Gouvernement. Chacun peut être mis aux voix séparément.",
          },
          {
            kind: 'p',
            texte:
              "C'est la raison principale du nombre de votes sur un même texte : l'essentiel des scrutins porte sur des amendements, pas sur la loi entière.",
          },
        ],
      },
      {
        id: 'q-motion',
        niveau: 'debutant',
        question: 'Qu’est-ce qu’une motion ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Une proposition qui porte sur le déroulement de l'examen, ou sur le Gouvernement, mais pas sur le contenu d'un texte.",
          },
          {
            kind: 'ul',
            items: [
              "<strong>Motion de rejet préalable</strong> — mise aux voix avant l'examen du fond ; adoptée, elle met fin à la discussion du texte.",
              "<strong>Motion de renvoi en commission</strong> — interrompt l'examen en séance pour renvoyer le texte à la commission.",
              '<strong>Motion de censure</strong> — met en cause la responsabilité du Gouvernement ; elle ne porte sur aucun texte.',
            ],
          },
          {
            kind: 'p',
            texte:
              "Une motion est mise aux voix comme le reste : elle donne lieu à un scrutin, avec son propre décompte. Un vote sur une motion n'est donc pas un vote sur le contenu du texte visé.",
          },
        ],
      },
      {
        id: 'q-commission-seance',
        niveau: 'debutant',
        question: 'Commission et séance publique : quelle différence ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "La commission examine le texte en comité restreint et le modifie avant son passage dans l'hémicycle. La séance publique réunit l'ensemble des députés.",
          },
          {
            kind: 'p',
            texte:
              "Les scrutins publiés sur ce site sont ceux de la séance publique. Les votes de commission ne figurent pas dans les données publiées par l'Assemblée.",
          },
        ],
      },
      {
        id: 'q-lecture',
        niveau: 'detaille',
        question: 'Que veut dire « première lecture » ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Un texte est examiné successivement par l'Assemblée et le Sénat. Chaque passage devant une assemblée est une lecture.",
          },
          {
            kind: 'p',
            texte:
              "Le texte fait la navette entre les deux jusqu'à un vote dans les mêmes termes. Le site n'affiche que la part Assemblée de ce parcours.",
          },
        ],
      },
      {
        id: 'q-texte-debattu',
        niveau: 'detaille',
        question: 'Qu’appelez-vous un « texte débattu » ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "C'est l'unité de regroupement propre à ce site : le texte que l'objet d'un scrutin nomme. Tous les scrutins qui nomment le même texte lui sont rattachés.",
          },
          {
            kind: 'p',
            texte:
              "Ce regroupement permet de suivre un texte de bout en bout même quand la source ne le relie à aucun dossier législatif. C'est aussi le texte, et non le dossier, qui porte le rattachement à un sujet.",
          },
        ],
      },
    ],
  },
  {
    id: 'scrutin',
    titre: 'Les scrutins',
    questions: [
      {
        id: 'q-scrutin',
        niveau: 'debutant',
        question: 'Qu’est-ce qu’un scrutin ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Un vote public, daté, sur un objet précis : un amendement, un article, une motion, ou l'ensemble d'un texte. La source publie le résultat et la position de chaque député.",
          },
          {
            kind: 'p',
            texte:
              "L'objet exact du scrutin est affiché en tête de chaque page de vote, dans les termes de la source. C'est lui qui définit ce sur quoi les députés se sont prononcés.",
          },
        ],
      },
      {
        id: 'q-scrutin-loi',
        niveau: 'debutant',
        question: 'Si un groupe a voté « contre », est-il contre la loi ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Pas nécessairement. Un scrutin porte rarement sur le texte entier : le plus souvent sur un amendement, un article, ou une question de procédure.",
          },
          {
            kind: 'p',
            texte:
              "Un même texte est mis aux voix des dizaines de fois : amendement par amendement, article par article, puis, le cas échéant, sur l'ensemble.",
          },
          {
            kind: 'p',
            texte:
              "Un groupe peut donc voter contre un amendement d'un texte qu'il approuve, et l'inverse. Pour connaître son vote sur l'ensemble, il faut chercher le scrutin dont l'objet est l'ensemble du texte.",
          },
          {
            kind: 'p',
            texte:
              "C'est la lecture la plus facile à se tromper sur ce site, et celle qui produit le plus de contresens quand un vote est cité isolément.",
          },
        ],
      },
      {
        id: 'q-scrutin-solennel',
        niveau: 'detaille',
        question: 'Scrutin ordinaire, scrutin solennel : quelle différence ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Le scrutin solennel est annoncé à l'avance et se tient à un moment fixé, en général pour le vote sur l'ensemble d'un texte. Le scrutin ordinaire intervient au fil de la discussion.",
          },
          {
            kind: 'p',
            texte:
              'Dans les deux cas, la source publie le décompte nominatif : le site les traite de la même façon.',
          },
        ],
      },
      {
        id: 'q-seconde-deliberation',
        niveau: 'detaille',
        question: 'Un même objet peut-il être voté deux fois ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Oui. Une seconde délibération demandée avant le vote sur l'ensemble produit un nouveau scrutin, avec son propre décompte. Les deux restent affichés : le second n'efface pas le premier.",
          },
        ],
      },
    ],
  },
  {
    id: 'positions',
    titre: 'Les positions de vote',
    questions: [
      {
        id: 'q-positions',
        niveau: 'debutant',
        question:
          'Que veulent dire « pour », « contre », « abstention », « non-votant » ?',
        reponse: [
          {
            kind: 'ul',
            items: [
              '<strong>Pour</strong> — le député approuve l’objet mis aux voix.',
              '<strong>Contre</strong> — il le rejette.',
              "<strong>Abstention</strong> — il prend part au scrutin sans se prononcer pour ni contre. L'abstention n'entre pas dans les suffrages exprimés, dont se déduit la majorité.",
              '<strong>Non-votant</strong> — il figure au scrutin sans que sa voix soit comptée.',
            ],
          },
        ],
      },
      {
        id: 'q-non-votant',
        niveau: 'debutant',
        question: 'Un « non-votant » est-il un député absent ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Non. La catégorie recouvre des situations de droit, pas un désintérêt ni une absence. Le président de séance, par exemple, ne prend pas part au vote qu'il préside.",
          },
          {
            kind: 'p',
            texte:
              'Le site ne publie pas de taux de présence, et un décompte de non-votants ne peut pas en tenir lieu.',
          },
        ],
      },
      {
        id: 'q-causes-non-vote',
        niveau: 'detaille',
        question: 'Comment savoir pourquoi un député est non-votant ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Le détail d'un scrutin affiche, pour chaque non-votant, la cause publiée par la source — présidence de séance, mission, ou tout autre motif qu'elle indique.",
          },
        ],
      },
      {
        id: 'q-delegation',
        niveau: 'detaille',
        question: 'Un député peut-il voter à la place d’un autre ?',
        reponse: [
          {
            kind: 'p',
            texte:
              'Un vote peut être émis par délégation : le député donne délégation à un collègue et la position est publiée à son nom. La source le signale, le site le reprend tel quel.',
          },
        ],
      },
      {
        id: 'q-mise-au-point',
        niveau: 'detaille',
        question: 'Que se passe-t-il si un député s’est trompé en votant ?',
        reponse: [
          {
            kind: 'p',
            texte:
              'Il peut demander une mise au point au procès-verbal, pour signaler après coup que la position publiée ne correspond pas à son intention. Elle est affichée avec le scrutin.',
          },
          {
            kind: 'p',
            texte:
              'Elle ne modifie jamais le décompte officiel, qui reste celui du vote. Le site affiche les deux, sans les mélanger.',
          },
        ],
      },
    ],
  },
  {
    id: 'dossier',
    titre: 'Les dossiers législatifs',
    questions: [
      {
        id: 'q-dossier',
        niveau: 'debutant',
        question: 'Qu’est-ce qu’un dossier législatif ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Le parcours administratif d'un texte, regroupé par la source : dépôt, examens, rapports, documents.",
          },
          {
            kind: 'p',
            texte:
              "C'est un classement documentaire, pas un vote : un dossier ne dit rien de ce que les députés ont décidé.",
          },
        ],
      },
      {
        id: 'q-sans-dossier',
        niveau: 'debutant',
        question: 'Pourquoi beaucoup de scrutins n’ont-ils aucun dossier ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Parce que la source publie ce lien de façon irrégulière — y compris entre deux scrutins d'un même texte. La majorité des scrutins ne porte donc aucun rattachement à un dossier.",
          },
          {
            kind: 'p',
            texte:
              "Ce n'est pas une lacune du site. C'est pourquoi le site classe par texte débattu et non par dossier : classer par dossier laisserait la majorité des votes hors thème, ce qui reviendrait à choisir ce que le lecteur voit.",
          },
        ],
      },
      {
        id: 'q-dossier-lien',
        niveau: 'detaille',
        question: 'Comment un scrutin est-il relié à un dossier ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Uniquement par le lien que la source publie elle-même. Le site ne devine aucun rapprochement à partir des libellés : un titre qui se ressemble ne suffit pas à établir un lien.",
          },
        ],
      },
      {
        id: 'q-dossier-vide',
        niveau: 'detaille',
        question: 'Que signifie un dossier sans aucun scrutin ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Que la source ne rattache aucun scrutin public à ce dossier. Cela ne veut pas dire qu'aucun vote n'a eu lieu sur ce texte : les votes peuvent exister sans porter le lien, ou avoir eu lieu à main levée.",
          },
          {
            kind: 'p',
            texte:
              "Ces dossiers restent consultables. Les masquer laisserait croire qu'ils n'existent pas.",
          },
        ],
      },
    ],
  },
  {
    id: 'groupe',
    titre: 'Les groupes parlementaires',
    questions: [
      {
        id: 'q-groupe',
        niveau: 'debutant',
        question: 'Un groupe parlementaire, est-ce un parti ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Non, et le site ne traduit jamais l'un en l'autre. Certains groupes rassemblent plusieurs partis, des députés y sont rattachés sans en être membres, et certains partis n'ont aucun groupe.",
          },
          {
            kind: 'p',
            texte:
              'Une équivalence approximative présentée comme un fait serait une information fausse. Le site affiche donc le groupe, nommé comme tel.',
          },
        ],
      },
      {
        id: 'q-changement-groupe',
        niveau: 'debutant',
        question: 'Et si un député change de groupe en cours de mandat ?',
        reponse: [
          {
            kind: 'p',
            texte:
              'Son vote reste compté avec le groupe qui était le sien à la date du scrutin, jamais avec son groupe actuel. Un changement de groupe ne réécrit pas les votes passés.',
          },
        ],
      },
      {
        id: 'q-apparente',
        niveau: 'detaille',
        question: 'Membre, apparenté, non-inscrit : quelle différence ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Un député peut être membre d'un groupe, y être apparenté, ou n'appartenir à aucun groupe — on parle alors de non-inscrit. La source publie ce statut, le site le reprend sans le simplifier.",
          },
        ],
      },
      {
        id: 'q-groupe-dissous',
        niveau: 'detaille',
        question: 'Et si un groupe disparaît en cours de législature ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Un groupe peut se constituer, changer de nom ou se dissoudre. Les scrutins antérieurs conservent la composition d'alors : c'est l'appartenance datée qui fait foi.",
          },
        ],
      },
    ],
  },
  {
    id: 'sorts',
    titre: 'Les résultats',
    questions: [
      {
        id: 'q-rejete',
        niveau: 'debutant',
        question: 'Pourquoi un scrutin est-il « rejeté » ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Parce que l'objet mis aux voix n'a pas réuni la majorité des suffrages exprimés. Le sort porte sur cet objet précis — un amendement, un article, une motion — et sur rien d'autre.",
          },
          {
            kind: 'p',
            texte:
              "Un rejet ne dit rien du contenu du texte ni des raisons du vote. Le site publie l'objet, le décompte et la source ; il n'attribue aucun motif.",
          },
        ],
      },
      {
        id: 'q-majorite',
        niveau: 'debutant',
        question: 'Comment la majorité est-elle calculée ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Un objet est adopté quand les voix « pour » sont majoritaires parmi les suffrages exprimés. Les abstentions n'entrent pas dans les exprimés.",
          },
          {
            kind: 'p',
            texte:
              'Un objet peut donc être adopté sans réunir la moitié des députés présents. Le nombre de voix requis est affiché sur chaque page de vote.',
          },
        ],
      },
      {
        id: 'q-egalite',
        niveau: 'detaille',
        question: 'Que se passe-t-il en cas d’égalité des voix ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "En cas de partage égal des suffrages, l'objet mis aux voix n'est pas adopté. Il n'existe pas de voix prépondérante.",
          },
        ],
      },
      {
        id: 'q-irrecevable',
        niveau: 'detaille',
        question: 'Pourquoi certains amendements ne sont-ils jamais votés ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Parce qu'ils sont déclarés irrecevables, et ne sont alors pas mis aux voix. L'irrecevabilité financière (article 40 de la Constitution) et l'irrecevabilité tirée du domaine de la loi ou du lien avec le texte (article 45) sont des filtres de procédure, sans vote public.",
          },
          {
            kind: 'p',
            texte:
              "Un amendement retiré par son auteur disparaît de la même manière. Conséquence : l'absence de scrutin sur un sujet ne signifie pas qu'aucun député ne l'a soulevé.",
          },
        ],
      },
    ],
  },
  {
    id: 'etapes',
    titre: 'Le parcours d’un texte',
    questions: [
      {
        id: 'q-adopte-promulgue',
        niveau: 'debutant',
        question: 'Un texte adopté à l’Assemblée est-il une loi ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Pas encore. Il doit être voté au Sénat dans les mêmes termes, puis promulgué. Le site affiche l'étape connue et sa date, sans préjuger de la suite.",
          },
        ],
      },
      {
        id: 'q-sans-rejet',
        niveau: 'detaille',
        question: 'Un texte peut-il s’arrêter sans avoir été rejeté ?',
        reponse: [
          {
            kind: 'ul',
            intro: 'Oui, de plusieurs façons :',
            items: [
              "le texte est adopté mais n'est jamais inscrit à l'ordre du jour du Sénat ;",
              'le texte est retiré par son auteur ou par le Gouvernement ;',
              'la législature s’achève et les textes non adoptés deviennent caducs ;',
              'le Conseil constitutionnel censure tout ou partie du texte après son adoption.',
            ],
          },
          {
            kind: 'p',
            texte:
              "Aucun de ces cas n'est présenté ici comme un échec ou une manœuvre : le site publie l'étape atteinte et sa date.",
          },
        ],
      },
    ],
  },
  {
    id: 'lacunes',
    titre: 'Ce que le site ne peut pas dire',
    questions: [
      {
        id: 'q-main-levee',
        niveau: 'debutant',
        question: 'Tous les votes de l’Assemblée sont-ils sur ce site ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Non. Les votes à main levée ne sont pas publiés par l'Assemblée — ni décompte, ni répartition. Ils sont absents du jeu de données, et le site ne peut donc rien en dire.",
          },
          {
            kind: 'p',
            texte:
              "Leur absence ne vaut pas absence de décision : elle signifie seulement qu'aucune trace nominative n'existe.",
          },
        ],
      },
      {
        id: 'q-candidats',
        niveau: 'debutant',
        question: 'Puis-je voir la position d’un candidat à la présidentielle ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Non. Une personnalité qui n'a jamais siégé à l'Assemblée n'a aucun vote ici, et cette absence n'est pas une information sur elle.",
          },
          {
            kind: 'p',
            texte:
              "Le site présente les votes d'un groupe parlementaire, jamais « la position » d'une personne.",
          },
        ],
      },
      {
        id: 'q-lacune-invisible',
        niveau: 'detaille',
        question: 'Comment repérer qu’un vote manque ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "On ne le repère pas depuis les données. Un vote à main levée ne laisse aucune trace dans le jeu de scrutins : l'absence n'y est pas marquée, elle est silencieuse.",
          },
          {
            kind: 'p',
            texte:
              "Le compte rendu de séance, publié séparément par l'Assemblée, retrace le déroulé des débats. Ce site n'en fait pas l'ingestion et ne peut donc pas combler cette lacune.",
          },
          {
            kind: 'p',
            texte:
              "C'est pourquoi la mention figure sur les listes de scrutins elles-mêmes, et pas seulement ici : une lacune reléguée dans une page d'explication passe pour une exhaustivité.",
          },
        ],
      },
      {
        id: 'q-scrutin-modifie',
        niveau: 'detaille',
        question: 'Un scrutin peut-il changer après sa publication ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Oui. La source peut republier un scrutin corrigé — une mise au point ajoutée après coup, par exemple. Le site réécrit alors intégralement sa version : c'est toujours l'état courant de la source qui est affiché.",
          },
          {
            kind: 'p',
            texte:
              "Une page consultée à deux dates peut donc différer. La source officielle, liée sur chaque scrutin, reste l'arbitre.",
          },
        ],
      },
    ],
  },
  {
    id: 'sources',
    titre: 'Les sources',
    questions: [
      {
        id: 'q-sources',
        niveau: 'debutant',
        question: 'D’où viennent les informations affichées ?',
        reponse: [
          {
            kind: 'p',
            texte:
              'De ces sources, et chaque page y renvoie. Aucun chiffre n’est produit par un modèle de langage.',
          },
          {
            kind: 'source',
            libelle: 'Open data de l’Assemblée nationale',
            lien: 'https://data.assemblee-nationale.fr/',
            precision:
              'Scrutins, positions nominales, députés, groupes et appartenances datées, dossiers législatifs. Licence Ouverte.',
          },
          {
            kind: 'source',
            libelle: 'Règlement de l’Assemblée nationale',
            lien: 'https://www.assemblee-nationale.fr/connaissance/reglement.asp',
            precision:
              'Procédure de séance : scrutin public, motions, irrecevabilités, mises au point au procès-verbal.',
          },
          {
            kind: 'source',
            libelle: 'Constitution du 4 octobre 1958',
            lien: 'https://www.conseil-constitutionnel.fr/le-bloc-de-constitutionnalite/texte-integral-de-la-constitution-du-4-octobre-1958-en-vigueur',
            precision:
              'Articles 40 et 45, cités pour les irrecevabilités qui empêchent la mise aux voix.',
          },
        ],
      },
      {
        id: 'q-verifier',
        niveau: 'detaille',
        question: 'Comment vérifier un chiffre affiché sur le site ?',
        reponse: [
          {
            kind: 'p',
            texte:
              "Chaque page de scrutin porte le lien vers sa page officielle à l'Assemblée. Le décompte affiché doit s'y retrouver à l'identique ; tout écart est un défaut du site, pas une interprétation.",
          },
          {
            kind: 'p',
            texte:
              "La méthode de rattachement d'un texte à un sujet est publiée à part, avec son taux de couverture et les textes qu'elle laisse non rattachés.",
          },
        ],
      },
    ],
  },
]

/** Toutes les questions à plat — sert à résoudre une ancre reçue en URL. */
export const QUESTIONS: Question[] = SECTIONS.flatMap((s) => s.questions)
