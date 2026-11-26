// Client de l'API GraphQL de Buffer (https://developers.buffer.com/reference) :
// lecture des posts étiquetés, dépôt des posts programmés, retrait des brouillons, étiquettes (REELS-PLAN-F4).

const ENDPOINT = 'https://api.buffer.com'
const PAGE_SIZE = 50
const MAX_ATTEMPTS = 3
/** Couleur des étiquettes créées par la chaîne (Buffer l'exige). */
const TAG_COLOR = '#0E7490'

const POST_FIELDS = 'id status dueAt channelId tags { id name }'

export class BufferError extends Error {}

export function bufferClient(apiKey) {
  if (!apiKey) throw new BufferError('BUFFER_API_KEY absent')

  async function graphql(query, variables = {}) {
    for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
      const response = await fetch(ENDPOINT, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${apiKey}` },
        body: JSON.stringify({ query, variables }),
      })
      if (response.status === 429 && attempt < MAX_ATTEMPTS) {
        const wait = Number(response.headers.get('retry-after') ?? '30')
        await new Promise((resolve) => setTimeout(resolve, Math.min(wait, 120) * 1000))
        continue
      }
      if (!response.ok) throw new BufferError(`Buffer a répondu ${response.status}`)
      const body = await response.json()
      if (body.errors?.length) {
        throw new BufferError(`Buffer : ${body.errors.map((error) => error.message).join(' ; ')}`)
      }
      return body.data
    }
    throw new BufferError('Buffer : limite de requêtes atteinte')
  }

  /** Résultat d'une mutation : succès attendu, sinon le message d'erreur de Buffer. */
  function expect(payload, field, action) {
    if (payload?.[field] !== undefined) return payload[field]
    throw new BufferError(`Buffer, ${action} : ${payload?.message ?? 'réponse inattendue'}`)
  }

  return {
    async organizationId() {
      const data = await graphql('query { account { organizations { id name } } }')
      const organizations = data.account.organizations
      if (organizations.length !== 1) {
        throw new BufferError(`Buffer : ${organizations.length} organisations, une seule attendue`)
      }
      return organizations[0].id
    },

    /** Étiquettes de l'organisation, par nom. */
    async tags(organizationId) {
      const byName = new Map()
      let after = null
      do {
        const data = await graphql(
          `query Tags($after: String, $input: TagsInput!) {
            tagsV2(first: 100, after: $after, input: $input) {
              edges { node { id name } }
              pageInfo { endCursor hasNextPage }
            }
          }`,
          { after, input: { organizationId } },
        )
        for (const { node } of data.tagsV2.edges) byName.set(node.name, node.id)
        after = data.tagsV2.pageInfo.hasNextPage ? data.tagsV2.pageInfo.endCursor : null
      } while (after)
      return byName
    },

    /** Identifiants des étiquettes nommées, créées si absentes. */
    async ensureTags(organizationId, names, known) {
      const ids = []
      for (const name of names) {
        if (!known.has(name)) {
          const data = await graphql(
            `mutation CreateTag($input: CreateTagInput!) {
              createTag(input: $input) {
                ... on TagActionSuccess { tag { id name } }
                ... on MutationError { message }
              }
            }`,
            { input: { organizationId, tag: { name, color: TAG_COLOR } } },
          )
          const tag = expect(data.createTag, 'tag', `création de l'étiquette ${name}`)
          known.set(tag.name, tag.id)
        }
        ids.push(known.get(name))
      }
      return ids
    },

    /** Posts filtrés (statut, canaux, étiquettes, échéance), au plus `max`. */
    async posts(organizationId, filter, max = 500) {
      const posts = []
      let after = null
      do {
        const data = await graphql(
          `query Posts($first: Int, $after: String, $input: PostsInput!) {
            posts(first: $first, after: $after, input: $input) {
              edges { node { ${POST_FIELDS} } }
              pageInfo { endCursor hasNextPage }
            }
          }`,
          {
            first: PAGE_SIZE,
            after,
            input: { organizationId, filter, sort: [{ field: 'dueAt', direction: 'desc' }] },
          },
        )
        posts.push(...data.posts.edges.map(({ node }) => node))
        after = data.posts.pageInfo.hasNextPage && posts.length < max ? data.posts.pageInfo.endCursor : null
      } while (after)
      return posts.slice(0, max)
    },

    /**
     * Post d'une vidéo (Reel) ou, avec `imageUrls`, d'un carrousel d'images
     * (post Instagram), programmé à `dueAt` : Buffer le publie seul à l'heure,
     * l'éditeur peut le relire, le corriger ou le repasser en brouillon d'ici là
     * (RM-13). Avec `draft`, il reste un brouillon que seul l'éditeur publie.
     */
    async schedulePost({ channelId, service, text, videoUrl, imageUrls, dueAt, tagIds, draft = false }) {
      const assets = imageUrls ? imageUrls.map((url) => ({ image: { url } })) : [{ video: { url: videoUrl } }]
      const metadata =
        service === 'instagram'
          ? { instagram: { type: imageUrls ? 'post' : 'reel', shouldShareToFeed: true } }
          : undefined
      const data = await graphql(
        `mutation CreatePost($input: CreatePostInput!) {
          createPost(input: $input) {
            ... on PostActionSuccess { post { id } }
            ... on MutationError { message }
          }
        }`,
        {
          input: {
            channelId,
            text,
            schedulingType: 'automatic',
            mode: 'customScheduled',
            dueAt,
            saveToDraft: draft,
            tagIds,
            assets,
            ...(metadata ? { metadata } : {}),
          },
        },
      )
      return expect(data.createPost, 'post', `post ${service}`).id
    },

    async deletePost(id) {
      const data = await graphql(
        `mutation DeletePost($input: DeletePostInput!) {
          deletePost(input: $input) {
            ... on DeletePostSuccess { id }
            ... on MutationError { message }
          }
        }`,
        { input: { id } },
      )
      return expect(data.deletePost, 'id', `retrait du brouillon ${id}`)
    },
  }
}
