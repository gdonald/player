import React, { useEffect, useRef } from 'react'

export default function Audio({
  src,
  doSrcEnded,
}: {
  src: string | null
  doSrcEnded: Function
}) {
  const audioRef = useRef<HTMLAudioElement | null>(null)

  const audioEnded = () => {
    doSrcEnded(true)
  }

  if (!src) {
    return <div></div>
  }

  return (
    <audio
      controls
      autoPlay
      src={src}
      preload='auto'
      ref={audioRef}
      onEnded={audioEnded}
    >
      Your browser does not support the <code>audio</code> element.
    </audio>
  )
}
