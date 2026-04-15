import { lazy, Suspense, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'

const ShaderSplash = lazy(() => import('../components/widgets/ShaderSplash'))

function hasWebGPU(): boolean {
  return 'gpu' in navigator
}

function VideoFallback() {
  return (
    <video
      autoPlay
      loop
      muted
      playsInline
      className="absolute inset-0 w-full h-full object-cover"
    >
      <source src="/cool_shader.mp4" type="video/mp4" />
    </video>
  )
}

export default function BusinessRedirect() {
  const navigate = useNavigate()
  const [supportsWebGPU] = useState(hasWebGPU)

  useEffect(() => {
    const timer = setTimeout(() => {
      navigate('/', { replace: true })
    }, 8000)
    return () => clearTimeout(timer)
  }, [navigate])

  return (
    <div className="fixed inset-0 z-[9999] bg-sf-black">
      {supportsWebGPU ? (
        <Suspense fallback={<VideoFallback />}>
          <ShaderSplash />
        </Suspense>
      ) : (
        <VideoFallback />
      )}
    </div>
  )
}
