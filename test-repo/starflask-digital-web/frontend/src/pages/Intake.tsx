import { Helmet } from 'react-helmet-async'
import Container from '../components/ui/Container'
import IntakeWizard from '../components/intake/IntakeWizard'

export default function Intake() {
  return (
    <>
      <Helmet>
        <title>Submit a Project — Starflask Digital</title>
      </Helmet>
      <div className="min-h-screen pt-24 pb-16 bg-gray-50">
        <Container>
          <div className="text-center mb-8">
            <h1 className="text-3xl sm:text-4xl font-heading font-bold text-gray-900">
              Let's Build Something <span className="text-accent">Great</span>
            </h1>
            <p className="mt-3 text-gray-500">Tell us about your project in a few simple steps.</p>
          </div>
          <IntakeWizard />
        </Container>
      </div>
    </>
  )
}
