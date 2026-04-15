import { useEffect, useRef } from 'react'
import { createTimeline, stagger, utils } from 'animejs'

export default function AnimeSplash() {
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!ref.current) return

    const tl = createTimeline({ defaults: { ease: 'outExpo' } })

    tl.add(ref.current.querySelector('.splash-img')!, {
      scale: [0.3, 1.05],
      opacity: [0, 1],
      rotate: [-8, 0],
      duration: 600,
    })
      .add(
        ref.current.querySelectorAll('.shard'),
        {
          scale: [0, 1],
          opacity: [0, 0.7],
          translateX: () => [utils.random(-400, 400), 0],
          translateY: () => [utils.random(-300, 300), 0],
          rotate: () => [utils.random(-180, 180), utils.random(-15, 15)],
          duration: 700,
          delay: stagger(60),
        },
        '-=300',
      )
      .add(
        ref.current.querySelector('.brand-text')!,
        { translateY: [40, 0], opacity: [0, 1], duration: 400 },
        '-=400',
      )
  }, [])

  const shards = Array.from({ length: 12 }, (_, i) => {
    const size = utils.random(40, 120)
    const hue = utils.random(140, 175)
    const lightness = utils.random(45, 65)
    return (
      <div
        key={i}
        className="shard absolute opacity-0"
        style={{
          width: size,
          height: size,
          top: `${utils.random(5, 85)}%`,
          left: `${utils.random(5, 85)}%`,
          background: `hsl(${hue}, 70%, ${lightness}%)`,
          clipPath: `polygon(${generatePolygon(utils.random(4, 6))})`,
        }}
      />
    )
  })

  return (
    <div ref={ref} className="absolute inset-0 flex items-center justify-center overflow-hidden">
      {shards}

      <img
        src="/cool_vectors.jpg"
        alt=""
        className="splash-img absolute w-[70vw] max-w-2xl opacity-0 object-contain pointer-events-none"
      />

      <span className="brand-text absolute bottom-[15%] text-white/90 text-2xl md:text-4xl font-bold tracking-widest uppercase opacity-0">
        Starflask Digital
      </span>
    </div>
  )
}

function generatePolygon(sides: number): string {
  const points: string[] = []
  for (let i = 0; i < sides; i++) {
    const angle = (Math.PI * 2 * i) / sides - Math.PI / 2
    const jitter = 0.15
    const r = 50 + utils.random(-50 * jitter, 50 * jitter)
    const x = 50 + r * Math.cos(angle)
    const y = 50 + r * Math.sin(angle)
    points.push(`${x.toFixed(1)}% ${y.toFixed(1)}%`)
  }
  return points.join(', ')
}
