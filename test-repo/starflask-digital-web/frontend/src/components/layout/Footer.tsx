import { Link, useLocation } from 'react-router-dom'
import Container from '../ui/Container'

export default function Footer() {
  const location = useLocation()
  if (location.pathname.startsWith('/admin')) return null

  return (
    <footer className="bg-sf-black text-gray-400 py-16">
      <Container>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-10">
          <div>
            <Link to="/" className="font-heading font-bold text-xl text-white">
              Starflask<span className="text-accent">.</span>
            </Link>
            <p className="mt-3 text-sm leading-relaxed">
              AI-powered digital solutions that transform how businesses operate.
            </p>
          </div>

          <div>
            <h4 className="text-white font-heading font-semibold mb-4">Services</h4>
            <ul className="space-y-2 text-sm">
              <li><a href="/#services" className="hover:text-accent transition-colors">Inbound Sales</a></li>
              <li><a href="/#services" className="hover:text-accent transition-colors">Web Design</a></li>
              <li><a href="/#services" className="hover:text-accent transition-colors">Agentic Automation</a></li>
            </ul>
          </div>

          <div>
            <h4 className="text-white font-heading font-semibold mb-4">Company</h4>
            <ul className="space-y-2 text-sm">
              <li><a href="/#about" className="hover:text-accent transition-colors">About</a></li>
              <li><a href="/#pillars" className="hover:text-accent transition-colors">Our Pillars</a></li>
              <li><a href="/#portfolio" className="hover:text-accent transition-colors">Portfolio</a></li>
            </ul>
          </div>

          <div>
            <h4 className="text-white font-heading font-semibold mb-4">Contact</h4>
            <ul className="space-y-2 text-sm">
              <li>
                <Link to="/intake" className="hover:text-accent transition-colors">Start a Project</Link>
              </li>
              <li>
                <a href="mailto:team@starflaskdigital.com" className="hover:text-accent transition-colors">
                  team@starflaskdigital.com
                </a>
              </li>
            </ul>
          </div>
        </div>

        <div className="border-t border-gray-800 mt-12 pt-8 text-center text-sm">
          <p>&copy; {new Date().getFullYear()} Starflask Digital. All rights reserved.</p>
        </div>
      </Container>
    </footer>
  )
}
