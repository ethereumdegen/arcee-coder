import { useState } from 'react'
import Container from '../ui/Container'
import ScrollReveal from '../animations/ScrollReveal'

const projects = [
  {
    title: 'Starflask',
    url: 'https://starflask.com',
    description:
      'An AI-powered SaaS platform that streamlines content workflows — generating presentations, documents, charts, and images from simple text prompts. Teams produce professional business content like slide decks, reports, and data visualizations in seconds from a web-based interface.',
  },
  {
    title: 'Octa Dashboard',
    url: 'https://github.com/ethereumdegen/octa-dashboard',
    description:
      'An open-source admin dashboard built in Rust with a pluggable microservice architecture. Features team management with GitHub OAuth, automatic service discovery and health monitoring, API key management, and analytics — all behind a single gateway.',
  },
  {
    title: 'Starfire CLI',
    url: 'https://github.com/ethereumdegen/starfire-cli',
    description:
      'A Rust-based CLI router and credential manager that lets developers register API keys once and automatically injects them into supported tools at runtime. Supports Wrangler, Vercel, Fly.io, Supabase, Neon, and more — keeping secrets out of shell history.',
  },
  {
    title: 'Image Dream Pro',
    url: 'https://github.com/ethereumdegen/image-dream-pro',
    description:
      'A cross-platform Electron desktop app for running fal.ai image generation models with a local-first media library. Users select from curated AI models, configure parameters, and generate images that are automatically saved and organized on disk.',
  },
]

export default function Founder() {
  const [activeIndex, setActiveIndex] = useState<number | null>(null)

  const active = activeIndex !== null ? projects[activeIndex] : null

  return (
    <section className="py-20 sm:py-28 bg-sf-black relative overflow-hidden">
      <div className="absolute inset-0 bg-gradient-to-b from-sf-dark via-sf-black to-sf-dark" />

      <Container className="relative z-10">
        <div className="grid lg:grid-cols-2 gap-16 items-center">
          {/* Left — Photo placeholder */}
          <ScrollReveal direction="right">
            <div className="flex items-center justify-center px-12 py-16">
              <div className="aspect-[4/5] w-3/4 max-w-xs rounded-3xl border border-gray-700/50 bg-sf-dark/40 backdrop-blur-sm overflow-hidden">
                <img
                  src="/biopic1.jpg"
                  alt="Andrew Mazzola"
                  className="w-full h-full object-cover"
                />
              </div>
            </div>
          </ScrollReveal>

          {/* Right — Bio + Tiles */}
          <ScrollReveal direction="left">
            <div>
              {/* Bio / project description — fixed height so nothing shifts */}
              <div className="h-64 mb-8">
                {active ? (
                  <div className="animate-fade-in">
                    <h2 className="text-4xl sm:text-5xl font-heading font-bold text-accent leading-tight mb-4">
                      {active.title}
                    </h2>
                    <p className="text-lg text-gray-300 leading-relaxed">
                      {active.description}
                    </p>
                  </div>
                ) : (
                  <div>
                    <h2 className="text-4xl sm:text-5xl font-heading font-bold text-white leading-tight mb-4">
                      Andrew <span className="text-accent">Mazzola</span>
                    </h2>
                    <p className="text-lg text-gray-400 leading-relaxed mb-2">
                      Software engineer since 2013 — building full-stack products,
                      developer tools, and AI-powered platforms. You have software challenges — I have solutions.
                    </p>
                    <p className="text-sm text-gray-500  ">
                      <a href="tel:7344449526" className="hover:text-accent transition-colors">
                        734-444-9526
                      </a>
                    </p>
                  </div>
                )}
              </div>

              {/* Project tiles */}
              <div className="grid grid-cols-2 gap-3">
                {projects.map((project, i) => {
                  const isActive = activeIndex === i
                  return (
                    <a
                      key={project.title}
                      href={project.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="group relative"
                      style={{ perspective: '600px' }}
                      onMouseEnter={() => setActiveIndex(i)}
                      onMouseLeave={() => setActiveIndex(null)}
                    >
                      <div
                        className="relative w-full transition-transform duration-500"
                        style={{
                          transformStyle: 'preserve-3d',
                          transform: isActive ? 'rotateY(180deg)' : 'rotateY(0deg)',
                        }}
                      >
                        {/* Front */}
                        <div
                          className="rounded-2xl border border-gray-700/50 bg-sf-dark
                            p-5 text-center"
                          style={{ backfaceVisibility: 'hidden' }}
                        >
                          <span className="text-sm font-heading font-semibold text-white">
                            {project.title}
                          </span>
                        </div>

                        {/* Back */}
                        <div
                          className="absolute inset-0 rounded-2xl border border-accent/40 bg-sf-dark
                            p-5 flex items-center justify-center"
                          style={{
                            backfaceVisibility: 'hidden',
                            transform: 'rotateY(180deg)',
                          }}
                        >
                          <span className="text-sm font-heading font-semibold text-accent">
                            View &rarr;
                          </span>
                        </div>
                      </div>
                    </a>
                  )
                })}
              </div>
            </div>
          </ScrollReveal>
        </div>
      </Container>
    </section>
  )
}
