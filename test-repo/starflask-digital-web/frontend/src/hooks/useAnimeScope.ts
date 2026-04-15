import { useRef, useEffect } from 'react'
import { remove } from 'animejs'

export function useAnimeScope() {
  const scopeRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    return () => {
      if (scopeRef.current) remove(scopeRef.current)
    }
  }, [])

  return scopeRef
}
