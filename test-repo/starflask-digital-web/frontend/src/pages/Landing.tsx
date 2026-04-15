import { Helmet } from 'react-helmet-async'
import Hero from '../components/sections/Hero'
import Founder from '../components/sections/Founder'
import Services from '../components/sections/Services'
import Pillars from '../components/sections/Pillars'
import About from '../components/sections/About'
import Portfolio from '../components/sections/Portfolio'
import CTA from '../components/sections/CTA'

export default function Landing() {
  return (
    <>
      <Helmet>
        <title>Starflask Digital — Digital Solutions Architect</title>
        <meta name="description" content="AI-powered solutions that transform how businesses operate. Inbound sales, web design, and agentic automation." />
      </Helmet>
      <Hero />
      <Founder />
      <Services />
      <Pillars />
      <About />
      <Portfolio />
      <CTA />
    </>
  )
}
