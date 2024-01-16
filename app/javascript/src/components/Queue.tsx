import React from 'react'
import { Mp3Type, QueuedMp3Type } from '../lib/types'

export default function Queue({
  queuedMp3s,
  currentQueuedMp3,
  playQueue,
  removeQueuedMp3,
  doSrcEnded,
  setSrc,
}: {
  queuedMp3s: QueuedMp3Type[]
  currentQueuedMp3: QueuedMp3Type | null
  playQueue: Function
  removeQueuedMp3: Function
  doSrcEnded: Function
  setSrc: Function
}) {
  if (!queuedMp3s) return <></>

  function doPlayQueue(e: React.MouseEvent<HTMLAnchorElement, MouseEvent>) {
    e.preventDefault()
    playQueue()
  }

  function deleteQueuedMp3(e: React.MouseEvent<HTMLAnchorElement, MouseEvent>) {
    e.preventDefault()
    const id = e.currentTarget.id

    if (currentQueuedMp3 && currentQueuedMp3.id === parseInt(id)) {
      doSrcEnded(true)
    }

    removeQueuedMp3(id)
    // setSrc(null)
  }

  function showPlayButton(): boolean {
    return !currentQueuedMp3 && queuedMp3s.length > 0
  }

  return (
    <div className=''>
      {showPlayButton() && (
        <div className='text-end pe-1 queue-play'>
          <a href='#' onClick={doPlayQueue}>
            <i className='bi-play-btn text-primary'></i>
          </a>
        </div>
      )}
      <div className=''>
        <table className='table table-striped table-hover mb-0'>
          <tbody>
            {queuedMp3s.map((queuedMp3) => (
              <tr
                className={`align-middle${
                  currentQueuedMp3 && currentQueuedMp3.id === queuedMp3['id']
                    ? ' table-primary'
                    : ''
                }`}
                key={`playlist_mp3-${queuedMp3['id']}`}
              >
                <td className='tight queue-delete-icon'>
                  <a
                    href='#'
                    onClick={deleteQueuedMp3}
                    id={queuedMp3['id'].toString()}
                  >
                    <i className='bi-trash text-primary'></i>
                  </a>
                </td>
                <td className='queue-title'>{queuedMp3['title']}</td>
                <td className='tight queue-current-icon'>
                  {currentQueuedMp3 &&
                  currentQueuedMp3.id === queuedMp3['id'] ? (
                    <i className='bi-arrow-left-circle-fill text-primary'></i>
                  ) : (
                    ''
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}
