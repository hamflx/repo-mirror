import { useState } from 'react'
import { useAsyncEffect } from '../../utils/useAsyncEffect'
import Repository from './Repository'

export function useRepositoryList() {
  const [repos, setRepos] = useState([])
  useAsyncEffect(async () => {
    setRepos(await (await fetch('/api/repos')).json())
  }, [])
  return {repos, setRepoProperty, removeRepository, addNewRepository}

  async function setRepoProperty (index, field, value) {
    const repo = repos[index]
    const old = repo[field]
    const revert = updateRepoTemporarily(copy => (copy[index][field] = value, copy))
    try {
      const result = await (await fetch(`/api/repo/${index}/${field}`, {
        method: 'POST',
        body: JSON.stringify({ value, old }),
        headers: { 'Content-Type': 'application/json' }
      })).json()
      if (!result) {
        throw new Error('Server returned an error')
      }
    } catch (e) {
      revert()
      throw e
    }
  }

  async function removeRepository (index) {
    const revert = updateRepoTemporarily(copy => (copy.splice(index, 1), copy))
    try {
      await fetch(`/api/repo/${index}`, { method: 'DELETE', })
    } catch (e) {
      revert()
      throw e
    }
  }

  async function addNewRepository () {
    const revert = updateRepoTemporarily(copy => (copy[copy.length] = { source: '', mirror: '' }, copy))
    try {
      await fetch(`/api/repo`, { method: 'POST' })
    } catch (e) {
      revert()
      throw e
    }
  }

  function updateRepoTemporarily (fn) {
    const copy = [...repos]
    const backup = repos.map(repo => ({ ...repo }))
    setRepos(fn(copy))
    return () => setRepos(backup)
  }
}

export default function RepositoryList({repos, addNewRepository, setRepoProperty, removeRepository}) {
  const items = repos.map((repo, index) => {
    return (
      <Repository
        repo={repo}
        setRepoProperty={(old, value) => setRepoProperty(index, old, value)}
        removeRepository={() => removeRepository(index)}
        key={index}
      />
    )
  })

  return (
    <div className="repository-list">
      <div className="repository-list__header">repository list</div>
      <div className="repository-list__content">
        {items}
        <span onClick={addNewRepository}>+</span>
      </div>
    </div>
  )
}
