import React, { useEffect, useState } from 'react'
import { SourceType, SourceErrorsType } from '../lib/types'

export default function Source({
  setShow,
  showMessage,
  showWait,
  id,
}: {
  setShow: Function
  showMessage: Function
  showWait: Function
  id: number
}) {
  const [source, setSource] = useState<SourceType | null>(null)
  const [errors, setErrors] = useState<SourceErrorsType | null>(null)

  const getSource = async () => {
    showWait(true)
    const req = await fetch(`/api/sources/${id}`)
    const data = await req.json()
    setSource(data.source)
    showWait(false)
  }

  useEffect(() => {
    getSource()
  }, [id])

  const save = async () => {
    if (!source) return

    showWait(true)
    const req = await fetch(`/api/sources/${id}`, {
      method: 'PUT',
      mode: 'cors',
      body: JSON.stringify({ source: { path: source.path } }),
      headers: {
        'Content-Type': 'application/json',
      },
    }).finally(() => {
      showWait(false)
    })

    const data = await req.json()

    if (data.errors) {
      setErrors(data.errors)
    } else {
      setSource(data.source)
      setErrors(null)
    }

    if (data.message) {
      showMessage(data.message)
    }
  }

  function saveSource(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    save()
  }

  function showSources(
    e: React.MouseEvent<HTMLAnchorElement> | React.MouseEvent<HTMLButtonElement>
  ) {
    e.preventDefault()
    showMessage('')
    setShow({ entity: 'sources', id: 0 })
  }

  if (!source) return <></>

  return (
    <div className='container-fluid'>
      <nav aria-label='breadcrumb'>
        <ol className='breadcrumb'>
          <li className='breadcrumb-item'>
            <a href='#' onClick={showSources} className='text-decoration-none'>
              <b>
                <i className='bi-folder'></i> Sources
              </b>
            </a>
          </li>
          <li className='breadcrumb-item' aria-current='page'>
            <b>{source['path']}</b>
          </li>
        </ol>
      </nav>

      <form className='form'>
        <div className='mb-3'>
          <label htmlFor='path' className='form-label'>
            Path
          </label>
          <input
            id='path'
            type='text'
            className='form-control'
            name='source[path]'
            defaultValue={source['path']}
            onChange={(e) => {
              setSource({ ...source, path: e.target.value })
            }}
            onBlur={(e) => {
              setSource({ ...source, path: e.target.value })
            }}
          />
          {errors && <div className='error'>{errors['path']}</div>}
        </div>

        <div className='btn-group' role='group' aria-label='Source actions'>
          <button
            type='submit'
            className='btn btn-primary'
            onClick={saveSource}
          >
            Save
          </button>
          <button
            type='submit'
            className='btn btn-primary'
            onClick={showSources}
          >
            Go Back
          </button>
        </div>
      </form>
    </div>
  )
}
