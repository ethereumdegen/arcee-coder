import { useState, useEffect } from 'react'
import { Link, useLocation } from 'react-router-dom'
import Button from '../ui/Button'
import MobileNav from './MobileNav'

const navLinks = [
  { label: 'Services', href: '/#services' },
  { label: 'Pillars', href: '/#pillars' },
  { label: 'About', href: '/#about' },
  { label: 'Portfolio', href: '/#portfolio' },
]

export default function Header() {
  const [scrolled, setScrolled] = useState(false)
  const [mobileOpen, setMobileOpen] = useState(false)
  const location = useLocation()

  const isLanding = location.pathname === '/'
  const isAdmin = location.pathname.startsWith('/admin')

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 50)
    window.addEventListener('scroll', onScroll)
    return () => window.removeEventListener('scroll', onScroll)
  }, [])

  if (isAdmin) return null

  const headerBg = scrolled || !isLanding
    ? 'bg-sf-black/90 backdrop-blur-md border-b border-gray-800/50'
    : 'bg-transparent'

  const textColor = 'text-white'

  return (
    <>
      <header className={`fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${headerBg}`}>
        <div className="max-w-7xl mx-auto px-6 lg:px-8 flex items-center justify-between h-16">
          <Link to="/" className={`font-heading font-bold text-xl ${textColor}`}>
            starflask <span className="text-accent">digital</span>
          </Link>

          <nav className="hidden md:flex items-center gap-8">
            {navLinks.map((link) => (
              <a
                key={link.href}
                href={link.href}
                className={`text-sm font-medium hover:text-accent transition-colors ${textColor}`}
              >
                {link.label}
              </a>
            ))}
          </nav>

          <div className="hidden md:block">
            <Link to="/login">
              <Button variant="filled" size="sm">
                Login
              </Button>
            </Link>
          </div>

          <button
            className={`md:hidden ${textColor} cursor-pointer`}
            onClick={() => setMobileOpen(true)}
            aria-label="Open menu"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
            </svg>
          </button>
        </div>
      </header>

      <MobileNav open={mobileOpen} onClose={() => setMobileOpen(false)} links={navLinks} />
    </>
  )
}
