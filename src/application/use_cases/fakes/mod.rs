//! Fakes partages des tests de use case: un fichier par port.
//!
//! Un fake dont le comportement n'appartient qu'a un test reste local, avec
//! une ligne `// Local: …` qui dit pourquoi.

pub mod theme_classifier;
pub mod theme_repository;
