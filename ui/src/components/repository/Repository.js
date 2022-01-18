import './Repository.css'

export default function Repository({repo, setRepoProperty, removeRepository}) {
  return (
    <div className="repository-item">
      <input
        className="repository-item__source"
        value={repo.source}
        onChange={event => setRepoProperty('source', event.target.value)}
      />
      <input
        className="repository-item__mirror"
        value={repo.mirror}
        onChange={event => setRepoProperty('mirror', event.target.value)}
      />
      <span onClick={removeRepository}>x</span>
    </div>
  )
}
