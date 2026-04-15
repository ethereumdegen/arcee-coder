import { useEffect, useRef } from 'react'
import { createTimeline, utils } from 'animejs'

export default function SuccessAnimation() {
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!ref.current) return

    const tl = createTimeline({ defaults: { ease: 'outExpo' } })

    tl.add(ref.current.querySelector('.circle')!, {
      scale: [0, 1],
      opacity: [0, 1],
      duration: 600,
    })
    .add(ref.current.querySelector('.check')!, {
      strokeDashoffset: [100, 0],
      duration: 800,
    }, '-=200')
    .add(ref.current.querySelectorAll('.particle'), {
      scale: [0, 1],
      opacity: [1, 0],
      translateX: () => utils.random(-80, 80),
      translateY: () => utils.random(-80, 80),
      duration: 1000,
    }, '-=400')
  }, [])

  return (
    <div ref={ref} className="relative w-32 h-32 mx-auto">
      <div className="circle absolute inset-0 rounded-full bg-accent/20 scale-0" />
      <svg className="absolute inset-0 w-full h-full" viewBox="0 0 64 64">
        <path
          className="check"
          d="M20 34l8 8 16-16"
          fill="none"
          stroke="#A78BFA"
          strokeWidth="3"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeDasharray="100"
          strokeDashoffset="100"
        />
      </svg>
      {[...Array(8)].map((_, i) => (
        <div
          key={i}
          className="particle absolute w-2 h-2 rounded-full bg-accent"
          style={{ top: '50%', left: '50%' }}
        />
      ))}
    </div>
  )
}
