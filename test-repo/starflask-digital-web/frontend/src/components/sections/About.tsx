import { Link } from 'react-router-dom'
import Container from '../ui/Container'
import Button from '../ui/Button'
import ScrollReveal from '../animations/ScrollReveal'

export default function About() {
  return (
    <section id="about" className="py-20 sm:py-28 bg-sf-dark relative overflow-hidden">
      <div className="absolute inset-0 bg-gradient-to-b from-sf-black via-sf-dark to-sf-black" />

      <Container className="relative z-10">
        <div className="grid lg:grid-cols-2 gap-16 items-center">
          <ScrollReveal direction="right">
            <div>
              <h2 className="text-4xl sm:text-5xl font-heading font-bold text-white leading-tight mb-6">
                From <span className="text-accent">concept</span> to{' '}
                <span className="text-accent">production</span>
              </h2>
              <p className="text-lg text-gray-400 leading-relaxed mb-6">
                We build software around the workflows you're already using — not the other way around.
                Your team shouldn't have to change how they work to fit a tool. We design AI-powered
                systems that plug into your existing processes, automate the tedious parts, and
                make everything run faster.
              </p>
              <p className="text-lg text-gray-400 leading-relaxed mb-8">
                Every project combines deep expertise in automation, prompt engineering,
                and system architecture to deliver production-grade solutions that handle
                edge cases, scale with your business, and pay for themselves.
              </p>
              <Link to="/intake">
                <Button variant="filled" size="md">Start a Conversation</Button>
              </Link>
            </div>
          </ScrollReveal>

          <ScrollReveal direction="left">
            <div className="relative">
              {/* Stats tiles hidden for now
              <div className="rounded-3xl border border-gray-700/50 bg-sf-black/60 backdrop-blur-sm p-2">
                <div className="grid grid-cols-2 gap-2">
                  {[
                    { value: '4+', label: 'Projects Delivered' },
                    { value: '95%', label: 'Client Retention' },
                    { value: '3x', label: 'Average ROI' },
                    { value: '24hr', label: 'Response Time' },
                  ].map((stat) => (
                    <div key={stat.label} className="p-8 rounded-2xl bg-sf-dark/80 border border-gray-800 text-center
                      hover:border-accent/30 transition-all duration-300">
                      <div className="text-4xl font-heading font-bold text-accent mb-2">
                        {stat.value}
                      </div>
                      <div className="text-sm text-gray-400">{stat.label}</div>
                    </div>
                  ))}
                </div>
              </div>
              */}
            </div>
          </ScrollReveal>
        </div>
      </Container>
    </section>
  )
}
