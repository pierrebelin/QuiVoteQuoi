/**
 * Première phrase d'un texte : jusqu'au premier point (ou « ! », « ? ») suivi
 * d'une espace ou de la fin. Sert à donner le périmètre d'un thème en une
 * ligne ; sans ponctuation finale, le texte est rendu tel quel.
 */
export function firstSentence(text: string): string {
  const trimmed = text.trim()
  const match = trimmed.match(/^[\s\S]*?[.!?…](?=\s|$)/)
  return match ? match[0] : trimmed
}
