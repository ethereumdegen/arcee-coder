import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { api, type BlogPost as BlogPostType } from '../lib/api'
import Container from '../components/ui/Container'

function renderMarkdown(md: string): string {
  return md
    .replace(/^## (.+)$/gm, '<h2 class="text-2xl font-heading font-bold text-white mt-10 mb-4">$1</h2>')
    .replace(/^### (.+)$/gm, '<h3 class="text-xl font-heading font-semibold text-white mt-8 mb-3">$1</h3>')
    .replace(/\*\*(.+?)\*\*/g, '<strong class="text-white font-semibold">$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/\[(.+?)\]\((.+?)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer" class="text-accent hover:underline">$1</a>')
    .replace(/^\d+\.\s+(.+)$/gm, '<li class="ml-6 list-decimal text-gray-300 mb-1">$1</li>')
    .replace(/^- (.+)$/gm, '<li class="ml-6 list-disc text-gray-300 mb-1">$1</li>')
    .replace(/\n\n/g, '</p><p class="text-gray-300 leading-relaxed mb-4">')
    .replace(/^(?!<[hl]|<li|<p)(.+)$/gm, '<p class="text-gray-300 leading-relaxed mb-4">$1</p>')
}

export default function BlogPost() {
  const { slug } = useParams<{ slug: string }>()
  const [post, setPost] = useState<BlogPostType | null>(null)
  const [error, setError] = useState(false)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (!slug) return
    api.getBlogPost(slug)
      .then((res) => setPost(res.post))
      .catch(() => setError(true))
      .finally(() => setLoading(false))
  }, [slug])

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-sf-black">
        <div className="w-8 h-8 border-2 border-accent border-t-transparent rounded-full animate-spin" />
      </div>
    )
  }

  if (error || !post) {
    return (
      <div className="min-h-screen flex flex-col items-center justify-center bg-sf-black text-white gap-4">
        <h1 className="text-2xl font-heading font-bold">Post not found</h1>
        <Link to="/" className="text-accent hover:underline">&larr; Back home</Link>
      </div>
    )
  }

  return (
    <main className="min-h-screen bg-sf-black pt-32 pb-20">
      <Container className="max-w-3xl">
        <Link to="/#portfolio" className="text-sm text-gray-400 hover:text-accent transition-colors mb-8 inline-block">
          &larr; Back to My Work
        </Link>

        <span className="text-sm font-semibold text-accent tracking-wide uppercase block mb-4">
          {post.category}
        </span>

        <h1 className="text-4xl sm:text-5xl font-heading font-bold text-white leading-tight mb-4">
          {post.title}
        </h1>

        <p className="text-gray-400 text-sm mb-10">
          {new Date(post.created_at).toLocaleDateString('en-US', {
            year: 'numeric',
            month: 'long',
            day: 'numeric',
          })}
        </p>

        <article
          className="prose-custom"
          dangerouslySetInnerHTML={{ __html: renderMarkdown(post.content) }}
        />
      </Container>
    </main>
  )
}
