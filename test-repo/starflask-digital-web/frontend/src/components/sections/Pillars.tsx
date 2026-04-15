import Container from '../ui/Container'
import ScrollReveal from '../animations/ScrollReveal'
import GlowLine from '../animations/GlowLine'

const pillars = [
  {
    num: '01',
    title: 'AI Workflow Design',
    rate: '',
    description:
      'Designing where AI handles what, where automation takes over, and where humans stay in the loop. Companies need this and have no idea how to do it themselves.',
  },
  {
    num: '02',
    title: 'Context Engineering',
    rate: '',
    description:
      'Building the data structures, references, and memory that make AI actually useful instead of generic. This is what separates AI that helps from AI that wastes time.',
  },
  {
    num: '03',
    title: 'No-Code Automation Architecture',
    rate: '',
    description:
      'Production systems in n8n or Make. Not toy workflows. Real systems that handle edge cases, run on schedules, and recover from failures.',
  },
  {
    num: '04',
    title: 'Production Prompt Engineering',
    rate: '',
    description:
      'Prompts that work reliably inside real systems. Consistent across thousands of inputs. Anyone can write a one-off prompt — almost nobody can write production prompts.',
  },
  {
    num: '05',
    title: 'AI System Architecture',
    rate: '',
    description:
      'Designing how AI, automation, and humans work together across an entire operation. Top of the pyramid — requires the other 4 skills plus business judgment.',
  },
]

export default function Pillars() {
  return (
    <section id="pillars" className="py-20 sm:py-28 bg-sf-black relative">
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,_rgba(167,139,250,0.04)_0%,_transparent_70%)]" />

      <Container className="relative z-10">
        <ScrollReveal>
          <div className="text-center mb-14  ">
            <h2 className="text-4xl sm:text-5xl font-heading font-bold text-white hidden">
              The <span className="text-accent">5 Pillars</span>
            </h2>
            <p className="mt-4 text-lg text-gray-400 max-w-2xl mx-auto">
              Five core competencies that define our expertise — the full stack of AI-powered business transformation.
            </p>
          </div>
        </ScrollReveal>

        <div className="max-w-3xl mx-auto space-y-4">
          {pillars.map((pillar, i) => (
            <ScrollReveal
              key={pillar.num}
              delay={i * 100}
              direction={i % 2 === 0 ? 'left' : 'right'}
            >
              <div className="relative group">
                <div className="flex items-start gap-6 p-8 rounded-2xl border border-gray-800 bg-sf-dark/50
                  hover:border-accent/30 hover:bg-sf-dark/80 transition-all duration-300">
                  <span className="text-5xl font-heading font-bold text-accent/30 group-hover:text-accent/50 shrink-0 transition-colors">
                    {pillar.num}
                  </span>
                  <div className="flex-1">
                    <h3 className="text-xl font-heading font-semibold text-white mb-3">
                      {pillar.title}
                    </h3>
                    <p className="text-gray-400 leading-relaxed text-[15px]">{pillar.description}</p>
                  </div>
                </div>

                {i < pillars.length - 1 && (
                  <div className="absolute left-12 -bottom-3 w-px h-6 bg-gradient-to-b from-accent/30 to-transparent" />
                )}
              </div>
            </ScrollReveal>
          ))}
        </div>

        <GlowLine className="mt-20" />
      </Container>
    </section>
  )
}
