import { Link } from 'react-router-dom'
import { AnimatePresence, motion } from 'framer-motion'
import Button from '../ui/Button'

interface MobileNavProps {
  open: boolean
  onClose: () => void
  links: { label: string; href: string }[]
}

export default function MobileNav({ open, onClose, links }: MobileNavProps) {
  return (
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 z-50 bg-sf-black/95 backdrop-blur-lg"
        >
          <div className="flex flex-col items-center justify-center h-full gap-8">
            <button
              onClick={onClose}
              className="absolute top-5 right-6 text-white cursor-pointer"
              aria-label="Close menu"
            >
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>

            {links.map((link) => (
              <a
                key={link.href}
                href={link.href}
                onClick={onClose}
                className="text-2xl font-heading font-semibold text-white hover:text-accent transition-colors"
              >
                {link.label}
              </a>
            ))}

            <Link to="/login" onClick={onClose}>
              <Button variant="filled" size="lg">
                Login
              </Button>
            </Link>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
