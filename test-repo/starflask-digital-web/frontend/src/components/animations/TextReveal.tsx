import { useEffect, useRef } from 'react'
import { animate, stagger } from 'animejs'
import { useInView } from '../../hooks/useInView'

interface TextRevealProps {
  children: string
  className?: string
  delay?: number
}

export default function TextReveal({ children, className = '', delay = 0 }: TextRevealProps) {
  const { ref: viewRef, inView } = useInView(0.3)
  const textRef = useRef<HTMLSpanElement>(null)

  useEffect(() => {
    if (!inView || !textRef.current) return

    const chars = textRef.current.querySelectorAll('.char')
    animate(chars, {
      opacity: [0, 1],
      translateY: [20, 0],
      delay: stagger(30, { start: delay }),
      ease: 'outExpo',
      duration: 800,
    })
  }, [inView, delay])

  const words = children.split(' ')

  return (
    <span ref={viewRef}>
      <span ref={textRef} className={className}>
        {words.map((word, wi) => (
          <span key={wi} className="inline-block whitespace-nowrap">
            {word.split('').map((char, ci) => (
              <span
                key={`${wi}-${ci}`}
                className="char inline-block opacity-0"
              >
                {char}
              </span>
            ))}
            {wi < words.length - 1 && <span className="char inline-block opacity-0">&nbsp;</span>}
          </span>
        ))}
      </span>
    </span>
  )
}
