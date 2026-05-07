import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import App from './App'
import { useRecordingStore } from './store'

// Mock the components so we don't need their full implementations
vi.mock('./components/RecorderView', () => ({
  RecorderView: () => <div data-testid="recorder-view">Recorder View Component</div>
}))

vi.mock('./components/LibraryView', () => ({
  LibraryView: () => <div data-testid="library-view">Library View Component</div>
}))

describe('App Component', () => {
  beforeEach(() => {
    // Clear the store state before each test
    useRecordingStore.setState({
      activeView: 'recorder',
      error: null,
      recordings: [],
      monitors: [],
      webcams: []
    })
  })

  it('renders without crashing and shows the Koom logo', () => {
    render(<App />)
    expect(screen.getByText('Koom')).toBeInTheDocument()
  })

  it('initially displays the RecorderView', () => {
    render(<App />)
    expect(screen.getByTestId('recorder-view')).toBeInTheDocument()
    expect(screen.queryByTestId('library-view')).not.toBeInTheDocument()
  })

  it('switches to LibraryView when Library button is clicked', () => {
    render(<App />)
    const libraryBtn = screen.getByText('Library')
    fireEvent.click(libraryBtn)
    
    expect(screen.getByTestId('library-view')).toBeInTheDocument()
    expect(screen.queryByTestId('recorder-view')).not.toBeInTheDocument()
  })

  it('displays error toast when there is an error in store', () => {
    useRecordingStore.setState({ error: 'Test error message' })
    render(<App />)
    expect(screen.getByText('Test error message')).toBeInTheDocument()
  })
})
