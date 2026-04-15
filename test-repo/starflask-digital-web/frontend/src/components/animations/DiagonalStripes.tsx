import { useEffect, useRef } from 'react'
import { animate, stagger } from 'animejs'

export default function DiagonalStripes({ className = '' }: { className?: string }) {
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!ref.current) return
    const stripes = ref.current.querySelectorAll('.stripe')
    animate(stripes, {
      translateX: ['100%', '0%'],
      opacity: [0, 1],
      delay: stagger(150, { start: 300 }),
      ease: 'outCubic',
      duration: 1200,
    })
  }, [])

  return (
    <div ref={ref} className={`absolute inset-0 overflow-hidden pointer-events-none ${className}`}>
      {[...Array(5)].map((_, i) => (
        <div
          key={i}
          className="stripe absolute opacity-0"
          style={{
            width: '120px',
            height: '400%',
            background: `linear-gradient(135deg, transparent 20%, rgba(167,139,250,${0.08 + i * 0.03}) 50%, transparent 80%)`,
            transform: `rotate(-35deg)`,
            right: `${i * 15 - 10}%`,
            top: '-50%',
          }}
        />
      ))}
    </div>
  )
}
