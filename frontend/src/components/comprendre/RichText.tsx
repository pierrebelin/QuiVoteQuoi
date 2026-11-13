/** Seul `<strong>` est admis dans le contenu de « Comprendre » — pas de HTML injecté. */
export function RichText({ texte }: { texte: string }) {
  const morceaux = texte.split(/<strong>(.*?)<\/strong>/g)
  return (
    <>
      {morceaux.map((morceau, index) =>
        index % 2 === 1 ? (
          <strong key={index} className="font-semibold text-ink">
            {morceau}
          </strong>
        ) : (
          morceau
        ),
      )}
    </>
  )
}
