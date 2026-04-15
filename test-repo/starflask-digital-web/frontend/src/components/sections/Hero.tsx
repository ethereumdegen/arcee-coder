import { useEffect, useRef } from 'react'
import { Link } from 'react-router-dom'
import { createTimeline, stagger } from 'animejs'
import Button from '../ui/Button'
import Container from '../ui/Container'
import ParticleField from '../animations/ParticleField'
import DiagonalStripes from '../animations/DiagonalStripes'
import FloatingOrbs from '../animations/FloatingOrbs'

export default function Hero() {
  const subtextRef = useRef<HTMLParagraphElement>(null)
  const buttonsRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const tl = createTimeline({ defaults: { ease: 'outExpo' } })

    tl.add(subtextRef.current ?? [], {
      opacity: [0, 1],
      translateY: [20, 0],
      duration: 800,
    })
    .add(buttonsRef.current?.children ?? [], {
      opacity: [0, 1],
      translateY: [20, 0],
      delay: stagger(100),
      duration: 600,
    }, '-=400')
  }, [])

  return (
    <section className="relative min-h-[100dvh] flex items-end pb-24 sm:items-center sm:pb-0 bg-sf-black overflow-hidden">
      <ParticleField />
      <DiagonalStripes />
      <FloatingOrbs />

      <Container className="relative z-10 pt-24">
        <div className="max-w-3xl">
          <p
            ref={subtextRef}
            className="text-lg sm:text-xl text-gray-300 max-w-xl mb-10 opacity-0"
          >
            We analyze business workflows to help optimize them around the ways you already work. From intelligent automation to higher-conversion funnels.
          </p>

          <div ref={buttonsRef} className="flex flex-wrap gap-4">
            <Link to="/intake">
              <Button variant="filled" size="lg">Submit a Project</Button>
            </Link>
            <a href="/#services">
              <Button variant="outlined" size="lg">Explore Services</Button>
            </a>
          </div>
        </div>
      </Container>

      <div className="hidden lg:block absolute right-10 top-1/2 -translate-y-1/2 z-10">
        <svg width="320" height="320" viewBox="0 0 320 320" fill="none" className="opacity-80">
          {/* Connecting lines */}
          <line x1="60" y1="80" x2="160" y2="40" className="stroke-accent/20" strokeWidth="1">
            <animate attributeName="opacity" values="0.2;0.5;0.2" dur="4s" repeatCount="indefinite" />
          </line>
          <line x1="160" y1="40" x2="260" y2="120" className="stroke-accent/20" strokeWidth="1">
            <animate attributeName="opacity" values="0.3;0.6;0.3" dur="3.5s" repeatCount="indefinite" />
          </line>
          <line x1="260" y1="120" x2="220" y2="240" className="stroke-accent/15" strokeWidth="1">
            <animate attributeName="opacity" values="0.2;0.4;0.2" dur="5s" repeatCount="indefinite" />
          </line>
          <line x1="220" y1="240" x2="100" y2="260" className="stroke-accent/20" strokeWidth="1">
            <animate attributeName="opacity" values="0.3;0.5;0.3" dur="4.5s" repeatCount="indefinite" />
          </line>
          <line x1="100" y1="260" x2="60" y2="80" className="stroke-accent/15" strokeWidth="1">
            <animate attributeName="opacity" values="0.2;0.5;0.2" dur="3s" repeatCount="indefinite" />
          </line>
          <line x1="60" y1="80" x2="220" y2="240" className="stroke-accent/10" strokeWidth="1">
            <animate attributeName="opacity" values="0.1;0.3;0.1" dur="6s" repeatCount="indefinite" />
          </line>
          <line x1="160" y1="40" x2="100" y2="260" className="stroke-accent/10" strokeWidth="1">
            <animate attributeName="opacity" values="0.1;0.25;0.1" dur="5.5s" repeatCount="indefinite" />
          </line>
          <line x1="160" y1="160" x2="160" y2="40" className="stroke-accent/15" strokeWidth="1">
            <animate attributeName="opacity" values="0.15;0.4;0.15" dur="4s" repeatCount="indefinite" />
          </line>
          <line x1="160" y1="160" x2="260" y2="120" className="stroke-accent/15" strokeWidth="1">
            <animate attributeName="opacity" values="0.2;0.35;0.2" dur="3.8s" repeatCount="indefinite" />
          </line>
          <line x1="160" y1="160" x2="60" y2="80" className="stroke-accent/15" strokeWidth="1">
            <animate attributeName="opacity" values="0.15;0.3;0.15" dur="4.2s" repeatCount="indefinite" />
          </line>

          {/* Nodes — outer ring */}
          <circle cx="60" cy="80" r="4" className="fill-accent/60">
            <animate attributeName="r" values="3;5;3" dur="3s" repeatCount="indefinite" />
          </circle>
          <circle cx="160" cy="40" r="5" className="fill-accent">
            <animate attributeName="r" values="4;6;4" dur="4s" repeatCount="indefinite" />
          </circle>
          <circle cx="260" cy="120" r="3.5" className="fill-accent/50">
            <animate attributeName="r" values="3;5;3" dur="3.5s" repeatCount="indefinite" />
          </circle>
          <circle cx="220" cy="240" r="4" className="fill-accent/40">
            <animate attributeName="r" values="3.5;5.5;3.5" dur="5s" repeatCount="indefinite" />
          </circle>
          <circle cx="100" cy="260" r="3" className="fill-accent/50">
            <animate attributeName="r" values="2.5;4;2.5" dur="4.5s" repeatCount="indefinite" />
          </circle>

          {/* Center node — larger, brighter */}
          <circle cx="160" cy="160" r="7" className="fill-accent/80">
            <animate attributeName="r" values="6;9;6" dur="3s" repeatCount="indefinite" />
          </circle>
          <circle cx="160" cy="160" r="16" className="fill-accent/10">
            <animate attributeName="r" values="14;22;14" dur="3s" repeatCount="indefinite" />
          </circle>

          {/* Glowing halos on key nodes */}
          <circle cx="160" cy="40" r="12" className="fill-accent/8">
            <animate attributeName="r" values="10;16;10" dur="4s" repeatCount="indefinite" />
          </circle>
          <circle cx="260" cy="120" r="10" className="fill-accent/6">
            <animate attributeName="r" values="8;14;8" dur="3.5s" repeatCount="indefinite" />
          </circle>

          {/* Traveling pulse along a line */}
          <circle r="2" className="fill-accent/70">
            <animateMotion dur="3s" repeatCount="indefinite" path="M60,80 L160,40 L260,120" />
          </circle>
          <circle r="1.5" className="fill-accent/50">
            <animateMotion dur="4s" repeatCount="indefinite" path="M260,120 L220,240 L100,260 L60,80" />
          </circle>
        </svg>
      </div>
    </section>
  )
}
