import React from 'react'
import { useState } from 'react'

export default function LoginForm({
  setAuthenticated,
}: {
  setAuthenticated: Function
}) {
  const [username, setUsername] = useState<string>('gd')
  const [password, setPassword] = useState<string>('changeme')

  const login = async () => {
    if (!username || !password) return

    const req = await fetch(`/api/sessions`, {
      method: 'POST',
      mode: 'cors',
      credentials: 'include',
      body: JSON.stringify({
        session: {
          username: username,
          password: password,
        },
      }),
      headers: {
        'Content-Type': 'application/json',
      },
    })
      .then((res) => {
        if (res.ok) {
          setAuthenticated(true)
        } else {
          setAuthenticated(false)
        }
      })
      .catch((err) => {
        console.log('err: ', err)
      })
  }

  function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault()
    login()
  }

  return (
    <div className='container-fluid'>
      <div className='row mt-5'>
        <div className='col-5'></div>

        <div className='col-2'>
          <form onSubmit={handleSubmit}>
            <div className='mb-3'>
              <label htmlFor='username' className='form-label'>
                Username
              </label>
              <input
                type='text'
                placeholder='Username'
                id='username'
                className='form-control'
                onChange={(e) => {
                  setUsername(e.target.value)
                }}
              />
            </div>
            <div className='mb-3'>
              <label htmlFor='password' className='form-label'>
                Password
              </label>
              <input
                type='password'
                placeholder='Password'
                id='password'
                className='form-control'
                onChange={(e) => {
                  setPassword(e.target.value)
                }}
              />
            </div>
            <button type='submit' className='btn btn-primary'>
              Login
            </button>
          </form>
        </div>
        <div className='col-5'></div>
      </div>
    </div>
  )
}
