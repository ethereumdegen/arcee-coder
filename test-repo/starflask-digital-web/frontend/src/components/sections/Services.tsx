import Container from '../ui/Container'
import ScrollReveal from '../animations/ScrollReveal'

const services = [
  {
    icon: (
      <svg className="w-10 h-10 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z" />
      </svg>
    ),
    title: 'Media Uploads & AI Transcription',
    description: 'Upload video, audio, or documents and let AI transcribe, summarize, and organize your content automatically.',
  },
  {
    icon: (
      <svg className="w-10 h-10 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V9a2 2 0 012-2h2a2 2 0 012 2v9a2 2 0 01-2 2h-2zM9 9h6M9 13h4" />
      </svg>
    ),
    title: 'Agentic Blog Posts & SEO',
    description: 'AI agents that research, write, and optimize blog content for search engines — driving organic traffic on autopilot.',
  },
  {
    icon: (
      <svg className="w-10 h-10 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
      </svg>
    ),
    title: 'Knowledgebase-Driven Agents',
    description: 'AI agents trained on your docs, SOPs, and internal knowledge — answering questions and executing tasks with full context.',
  },
  {
    icon: (
      <svg className="w-10 h-10 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M3 3h2l.4 2M7 13h10l4-8H5.4M7 13L5.4 5M7 13l-2.293 2.293c-.63.63-.184 1.707.707 1.707H17m0 0a2 2 0 100 4 2 2 0 000-4zm-8 2a2 2 0 100 4 2 2 0 000-4z" />
      </svg>
    ),
    title: 'Inbound Sales Solutions',
    description: 'High-converting funnels, lead capture systems, and CRM integrations that turn visitors into customers on autopilot.',
  },
  {
    icon: (
      <svg className="w-10 h-10 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
      </svg>
    ),
    title: 'Web Design',
    description: 'Modern, pixel-perfect websites that combine stunning visuals with performance. Built to convert and scale.',
  },
  {
    icon: (
      <svg className="w-10 h-10 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
      </svg>
    ),
    title: 'Agentic Automation',
    description: 'AI agents and intelligent workflows that handle complex business processes. Replace manual work with systems that think.',
  },
]

export default function Services() {
  return (
    <section id="services" className="py-20 sm:py-28 bg-sf-dark relative overflow-hidden">
      <div className="absolute inset-0 bg-gradient-to-b from-sf-black to-sf-dark" />

      <Container className="relative z-10">
        <ScrollReveal>
          <div className="text-center mb-14">
            <h2 className="text-4xl sm:text-5xl font-heading font-bold text-white">
              What We <span className="text-accent">Build</span>
            </h2>
            <p className="mt-4 text-lg text-gray-400 max-w-2xl mx-auto">
              Digital transformation powered by AI, built to deliver measurable results.
            </p>
          </div>
        </ScrollReveal>

        <div className="grid md:grid-cols-3 gap-8">
          {services.map((service, i) => (
            <ScrollReveal key={service.title} delay={i * 150}>
              <div className="h-full text-center p-8 rounded-2xl border border-gray-700/50 bg-sf-black/60 backdrop-blur-sm
                hover:border-accent/40 hover:shadow-[0_0_30px_rgba(167,139,250,0.08)] transition-all duration-300 group">
                <div className="mb-6 flex justify-center">
                  <div className="w-16 h-16 rounded-xl bg-accent/10 flex items-center justify-center group-hover:bg-accent/20 transition-colors">
                    {service.icon}
                  </div>
                </div>
                <h3 className="text-xl font-heading font-semibold text-white mb-3">
                  {service.title}
                </h3>
                <p className="text-gray-400 leading-relaxed">{service.description}</p>
              </div>
            </ScrollReveal>
          ))}
        </div>
      </Container>
    </section>
  )
}
