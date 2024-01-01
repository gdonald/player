import React from 'react'

export default function Alert({
  message,
  setMessage,
}: {
  message: string
  setMessage: Function
}) {
  if (!message) return <></>

  return (
    <div className='px-2'>
      <div
        className='alert alert-primary alert-dismissible fade show'
        role='alert'
      >
        {message}
        <button
          type='button'
          className='btn-close'
          aria-label='Close'
          onClick={() => setMessage('')}
        ></button>
      </div>
    </div>
  )
}
