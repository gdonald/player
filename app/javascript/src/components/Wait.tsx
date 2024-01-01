import React from 'react'

export default function Wait({ wait }: { wait: boolean }) {
  return (
    <div className='wait'>
      {wait && (
        <div className='waiting'>
          <div></div>
          <div></div>
        </div>
      )}
    </div>
  )
}
