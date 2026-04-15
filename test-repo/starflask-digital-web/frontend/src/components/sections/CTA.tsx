import { Link } from 'react-router-dom'
import Container from '../ui/Container'
import Button from '../ui/Button'
import ScrollReveal from '../animations/ScrollReveal'

export default function CTA() {
  return (
    <section className="py-20 sm:py-28 bg-sf-dark relative overflow-hidden">
      <div className="absolute inset-0 bg-gradient-to-r from-accent/5 via-transparent to-accent/5 animate-pulse-glow" />
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,_rgba(167,139,250,0.08)_0%,_transparent_60%)]" />

      <Container className="relative z-10">
        <ScrollReveal>
          <div className="text-center max-w-2xl mx-auto">
            <p className="text-lg text-gray-400 max-w-xl mx-auto mb-10">
              Tell us about your project. Get a response in 24 hr.
            </p>
            <Link to="/intake">
              <Button variant="filled" size="lg">Submit a Project</Button>
            </Link>
          </div>
        </ScrollReveal>
      </Container>
    </section>
  )
}
