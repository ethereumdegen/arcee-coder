import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import Container from '../ui/Container'
import ScrollReveal from '../animations/ScrollReveal'
import { api, type BlogPost } from '../../lib/api'

const blogCoverImages = [
  '/imagedream_perspective_1.png',
  '/starflask_perspective_1.png',
]

export default function Portfolio() {
  const [posts, setPosts] = useState<BlogPost[]>([])

  useEffect(() => {
    api.getBlogPosts()
      .then((res) => setPosts(res.posts.slice(0, 2)))
      .catch(() => {})
  }, [])

  return (
    <section id="portfolio" className="py-20 sm:py-28 bg-sf-black relative">
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top,_rgba(167,139,250,0.03)_0%,_transparent_50%)]" />

      <Container className="relative z-10">
        <ScrollReveal>
          <div className="text-center mb-14">
            <h2 className="text-4xl sm:text-5xl font-heading font-bold text-white">
              My <span className="text-accent">Work</span>
            </h2>
            <p className="mt-4 text-lg text-gray-400 max-w-2xl mx-auto">
              A selection of projects and case studies that showcase what happens when strategy meets execution.
            </p>
          </div>
        </ScrollReveal>

        <div className="grid md:grid-cols-2 gap-6">
          {/* Blog post cards */}
          {posts.map((post, i) => (
            <ScrollReveal key={post.slug} delay={i * 100}>
              <Link
                to={`/blog/${post.slug}`}
                className={`group relative overflow-hidden rounded-2xl block border border-gray-800 hover:border-violet-500/40 transition-all duration-300`}
              >
                <div
                  className="aspect-[4/3] relative overflow-hidden bg-sf-dark/40 bg-cover bg-center"
                  style={{ backgroundImage: blogCoverImages[i] ? `url(${blogCoverImages[i]})` : undefined }}
                >
                  <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/40 to-black/10" />
                  <div className="relative z-10 h-full p-8 flex flex-col justify-end">
                    <span className="text-sm font-semibold text-accent mb-2 tracking-wide uppercase">{post.category}</span>
                    <h3 className="text-2xl font-heading font-semibold text-white mb-2">
                      {post.title}
                    </h3>
                    <p className="text-gray-300 transform translate-y-4 opacity-0 group-hover:translate-y-0 group-hover:opacity-100 transition-all duration-300">
                      {post.summary}
                    </p>
                  </div>
                </div>
              </Link>
            </ScrollReveal>
          ))}
        </div>
      </Container>
    </section>
  )
}
