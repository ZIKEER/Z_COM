import time

from src.io.rtt_reader import RttReaderThread
from src.io.serial_reader import SerialReaderThread


class _TimedSerialPort:
    def __init__(self, chunks, interval):
        self._chunks = list(chunks)
        self._interval = interval
        self._start = time.monotonic()
        self._index = 0
        self.is_open = True

    @property
    def in_waiting(self):
        if self._index >= len(self._chunks):
            return 0
        if (time.monotonic() - self._start) >= (self._index * self._interval):
            return len(self._chunks[self._index])
        return 0

    def read(self, _size):
        data = self._chunks[self._index]
        self._index += 1
        return data


class _TimedJLink:
    def __init__(self, chunks, interval):
        self._chunks = list(chunks)
        self._interval = interval
        self._start = time.monotonic()
        self._index = 0

    def opened(self):
        return True

    def rtt_read(self, _buffer_idx, _read_size):
        if self._index >= len(self._chunks):
            return b""
        if (time.monotonic() - self._start) >= (self._index * self._interval):
            data = self._chunks[self._index]
            self._index += 1
            return data
        return b""


class TestReaderThreads:
    def test_serial_reader_flushes_continuous_stream_without_idle_gap(self, qapp, qtbot):
        port = _TimedSerialPort([b"A"] * 8, interval=0.03)
        reader = SerialReaderThread(port, frame_timeout=50)
        received = []
        emit_times = []
        start = time.monotonic()

        reader.data_received.connect(lambda data: (received.append(data), emit_times.append(time.monotonic())))
        reader.start()

        qtbot.waitUntil(lambda: len(emit_times) >= 1, timeout=400)
        qtbot.waitUntil(lambda: port._index == len(port._chunks), timeout=400)
        qtbot.wait(120)
        reader.stop()

        assert emit_times[0] - start < 0.18
        assert b"".join(received) == b"A" * 8

    def test_rtt_reader_flushes_continuous_stream_without_idle_gap(self, qapp, qtbot):
        jlink = _TimedJLink([b"B"] * 8, interval=0.03)
        reader = RttReaderThread(jlink, frame_timeout=0.05)
        received = []
        emit_times = []
        start = time.monotonic()

        reader.data_received.connect(lambda data: (received.append(data), emit_times.append(time.monotonic())))
        reader.start()

        qtbot.waitUntil(lambda: len(emit_times) >= 1, timeout=400)
        qtbot.waitUntil(lambda: jlink._index == len(jlink._chunks), timeout=400)
        qtbot.wait(120)
        reader.stop()

        assert emit_times[0] - start < 0.18
        assert b"".join(received) == b"B" * 8
