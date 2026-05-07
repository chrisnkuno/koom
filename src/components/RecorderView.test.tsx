import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { RecorderView } from './RecorderView'
import { useRecordingStore } from '../store'

// Mock the framer-motion components to avoid animation complexity in tests
vi.mock('framer-motion', () => ({
  motion: {
    div: ({ children, ...props }: any) => <div {...props}>{children}</div>,
    span: ({ children, ...props }: any) => <span {...props}>{children}</span>,
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}))

describe('RecorderView Component', () => {
  beforeEach(() => {
    // Set a predictable initial state
    useRecordingStore.setState({
      status: 'idle',
      elapsedSecs: 0,
      sessionName: null,
      monitors: [{ index: 0, name: 'Primary Monitor', width: 1920, height: 1080, is_primary: true }],
      webcams: [{ index: 0, name: 'Logitech C920' }],
      selectedMonitor: 0,
      selectedWebcam: 0,
      micEnabled: true,
      webcamEnabled: false,
      fps: 30,
      webcamCorner: 'br',
      startRecording: vi.fn(),
      stopRecording: vi.fn(),
      setMonitor: vi.fn(),
      setWebcam: vi.fn(),
      setMicEnabled: vi.fn(),
      setWebcamEnabled: vi.fn(),
      setFps: vi.fn(),
      setWebcamCorner: vi.fn(),
    })
  })

  it('renders correctly with default state', () => {
    const renderResult = render(<RecorderView />)
    
    // Header should be present
    expect(screen.getByText('New Recording')).toBeInTheDocument()
    
    // Status should be Ready
    expect(screen.getByText('Ready')).toBeInTheDocument()
    
    // Initial FPS
    expect(screen.getByText('30 fps')).toBeInTheDocument()
    
    // Mic is on by default
    const micBtn = renderResult.container.querySelector('#toggle-mic')
    expect(micBtn).toHaveClass('on')
    
    // Webcam is off by default
    const webcamBtn = renderResult.container.querySelector('#toggle-webcam')
    expect(webcamBtn).not.toHaveClass('on')
  })

  it('toggles webcam correctly', () => {
    const setWebcamEnabledMock = vi.fn()
    useRecordingStore.setState({ setWebcamEnabled: setWebcamEnabledMock })
    
    const { container } = render(<RecorderView />)
    
    const webcamBtn = container.querySelector('#toggle-webcam') as Element
    fireEvent.click(webcamBtn)
    
    expect(setWebcamEnabledMock).toHaveBeenCalledWith(true)
  })

  it('starts recording when record button is clicked', () => {
    const startRecordingMock = vi.fn()
    useRecordingStore.setState({ startRecording: startRecordingMock })
    
    render(<RecorderView />)
    
    const recordBtn = screen.getByTitle('Start recording')
    fireEvent.click(recordBtn)
    
    expect(startRecordingMock).toHaveBeenCalled()
  })
})
