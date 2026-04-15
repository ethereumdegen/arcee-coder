import { useEffect, useRef, type ReactNode } from 'react'
import { animate } from 'animejs'
import { useInView } from '../../hooks/useInView'

interface ScrollRevealProps {
  children: ReactNode
  className?: string
  delay?: number
  direction?: 'up' | 'left' | 'right'
}

export default function ScrollReveal({
  children,
  className = '',
  delay = 0,
  direction = 'up',
}: ScrollRevealProps) {
  const { ref, inView } = useInView(0.15)
  const innerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!inView || !innerRef.current) return

    const translateProp = direction === 'up' ? 'translateY' : 'translateX'
    const startVal = direction === 'right' ? -40 : 40

    animate(innerRef.current, {
      opacity: [0, 1],
      [translateProp]: [startVal, 0],
      delay,
      ease: 'outCubic',
      duration: 800,
    })
  }, [inView, delay, direction])

  return (
    <div ref={ref} className={className}>
      <div ref={innerRef} style={{ opacity: 0 }}>
        {children}
      </div>
    </div>
  )
}
