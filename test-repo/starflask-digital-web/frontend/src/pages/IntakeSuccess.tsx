import { Link } from 'react-router-dom'
import { Helmet } from 'react-helmet-async'
import Container from '../components/ui/Container'
import Button from '../components/ui/Button'
import SuccessAnimation from '../components/intake/SuccessAnimation'

export default function IntakeSuccess() {
  return (
    <>
      <Helmet>
        <title>Submitted — Starflask Digital</title>
      </Helmet>
      <div className="min-h-screen pt-24 pb-16 bg-gray-50 flex items-center">
        <Container>
          <div className="text-center max-w-lg mx-auto">
            <SuccessAnimation />
            <h1 className="text-3xl font-heading font-bold text-gray-900 mt-8 mb-4">
              We Got Your Project!
            </h1>
            <p className="text-lg text-gray-500 mb-8">
              Thanks for reaching out. We'll review your submission and get back to you within 24 hours.
            </p>
            <Link to="/">
              <Button variant="dark">Back to Home</Button>
            </Link>
          </div>
        </Container>
      </div>
    </>
  )
}
