import React, { useEffect, useState } from 'react'
import { SourceType } from '../lib/types'

export default function Sources({
  setShow,
  showMessage,
  showWait,
}: {
  setShow: Function
  showMessage: Function
  showWait: Function
}) {
  const [sources, setSources] = useState<SourceType[]>([])

  const getSources = async () => {
    showWait(true)
    await fetch('/api/sources')
      .then((req) => {
        if (!req.ok) {
          window.location.href = '/'
        }

        return req.json()
      })
      .then((data) => {
        setSources(data.sources)
      })
      .finally(() => {
        showWait(false)
      })
  }

  useEffect(() => {
    getSources()
  }, [])

  function editSource(e: React.MouseEvent<HTMLButtonElement>) {
    e.preventDefault()
    setShow({ entity: 'source', id: e.currentTarget.id })
  }

  function scanSource(e: React.MouseEvent<HTMLButtonElement>) {
    showWait(true)

    fetch(`/api/sources/${e.currentTarget.id}/scan`)
      .then((res) => {
        if (res.ok) {
          showMessage('Source scanning has been scheduled')
        } else {
          showMessage('Source scanning failed')
        }
      })
      .finally(() => {
        showWait(false)
      })
  }

  return (
    <div className='container-fluid'>
      <nav aria-label='breadcrumb'>
        <ol className='breadcrumb'>
          <li className='breadcrumb-item'>
            <b>
              <i className='bi-folder'></i> Sources
            </b>
          </li>
        </ol>
      </nav>

      {sources.length === 0 && (
        <p className='text-center pt-5'>No sources found.</p>
      )}

      {sources.length > 0 && (
        <table className='table table-striped table-hover'>
          <thead>
            <tr>
              <th>Path</th>
              <th>MP3s</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {sources.map((source) => (
              <tr key={`source-${source['id']}`} className='align-middle'>
                <td>{source['path']}</td>
                <td>{source['mp3s_count']}</td>
                <td className='tight'>
                  <div
                    className='btn-group'
                    role='group'
                    aria-label='Basic example'
                  >
                    <button
                      className='btn btn-sm btn-primary'
                      onClick={editSource}
                      id={source['id'].toString()}
                    >
                      Edit
                    </button>
                    <button
                      className='btn btn-sm btn-primary'
                      onClick={scanSource}
                      id={source['id'].toString()}
                    >
                      Scan
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  )
}
