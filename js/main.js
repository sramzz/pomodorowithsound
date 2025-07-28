// DOM Elements
const timerDisplay = document.getElementById('timer');
const startBtn = document.getElementById('startBtn');
const resetBtn = document.getElementById('resetBtn');
const endBtn = document.getElementById('endBtn');
const durationSelect = document.getElementById('durationSelect');
const goalInput = document.getElementById('goalInput');
const sessionLogBody = document.getElementById('sessionLogBody');
const jsonOutput = document.getElementById('jsonOutput');
const completionModal = document.getElementById('completionModal');
const closeModalBtn = document.getElementById('closeModalBtn');
const testSoundBtn = document.getElementById('testSoundBtn');
const clearAllBtn = document.getElementById('clearAllBtn');

// Timer State
let targetTime; // The exact timestamp when the timer should end
let timeLeft = 25 * 60; // Default to 25 minutes in seconds
let isRunning = false;
let animationFrameId; // For updating the visual display
let timeoutId; // For triggering the end of the timer reliably
let duration = 25 * 60; // Default duration in seconds
let currentSession = null;
let sessions = [];
let synth; // To store the synth instance for continuous sound

// --- LOCAL STORAGE & DATA HANDLING ---

/**
 * Checks if localStorage is available to prevent crashes in restricted environments.
 */
function isLocalStorageAvailable() {
    try {
        localStorage.setItem('test', 'test');
        localStorage.removeItem('test');
        return true;
    } catch (e) {
        return false;
    }
}

/**
 * Loads sessions from localStorage when the page loads.
 */
function loadSessions() {
    if (isLocalStorageAvailable()) {
        const storedSessions = localStorage.getItem('pomodoroSessions');
        if (storedSessions) {
            sessions = JSON.parse(storedSessions);
        }
    } else {
        console.warn('localStorage is not available. Session data will not persist.');
    }
    updateSessionLog();
}

/**
 * Saves the current list of sessions to localStorage.
 */
function saveSessions() {
    if (isLocalStorageAvailable()) {
        localStorage.setItem('pomodoroSessions', JSON.stringify(sessions));
    }
}

// --- UTILITY FUNCTIONS ---

/**
 * Formats a date object into a readable string (e.g., "06/11/2025, 11:15:30 AM").
 * @param {string} date - The ISO date string to format.
 */
function formatDate(date) {
    return new Date(date).toLocaleString('en-US', {
        month: '2-digit', day: '2-digit', year: 'numeric',
        hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: true
    });
}

/**
 * Formats a duration in seconds to a "Xm Ys" string.
 * @param {number} seconds - The duration in seconds.
 */
function formatDuration(seconds) {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
}

/**
 * Creates a readable summary of pause intervals.
 * @param {Array} pauses - An array of pause objects.
 */
function formatPauses(pauses) {
    if (!pauses || pauses.length === 0) return 'None';
    return pauses.map(pause =>
        `Paused for ${Math.round((new Date(pause.resumeTime) - new Date(pause.pauseTime)) / 1000)}s`
    ).join('<br>');
}

// --- NOTIFICATIONS & SOUND ---

/**
 * Asks the user for permission to show notifications if not already granted.
 */
function requestNotificationPermission() {
    if ('Notification' in window && Notification.permission === 'default') {
        Notification.requestPermission();
    }
}

/**
 * Shows a system notification to the user.
 */
function showSystemNotification() {
    if ('Notification' in window && Notification.permission === 'granted') {
        new Notification('Pomodoro Complete!', {
            body: `Your session for "${currentSession?.goal || 'your task'}" has finished.`,
            icon: 'https://img.icons8.com/plasticine/100/tomato.png' // A generic tomato icon
        });
    }
}

/**
 * Plays a notification sound using Tone.js.
 */
function playSound() {
    if (typeof Tone === 'undefined') return;
    // Ensure the audio context is running, as browsers require user interaction to start it.
    if (Tone.context.state !== 'running') {
        Tone.context.resume();
    }
    // Create a simple synthesizer and connect it to the main output (your speakers).
    // Store the synth instance to control it later (e.g., stop it).
    synth = new Tone.Synth().toDestination();

    // Create a loop for the sound.
    // The loop will play two notes and then repeat every 1 second.
    new Tone.Loop(time => {
        synth.triggerAttackRelease("C5", "8n", time);
        synth.triggerAttackRelease("G5", "8n", time + 0.2);
    }, "1s").start(0); // Start the loop immediately

    // Start Tone.Transport which is used by Tone.Loop
    Tone.Transport.start();
}

/**
 * Plays a single instance of the notification sound for testing.
 */
function playTestSound() {
    if (typeof Tone === 'undefined') return;
    if (Tone.context.state !== 'running') {
        Tone.context.resume();
    }
    const testSynth = new Tone.Synth().toDestination();
    testSynth.triggerAttackRelease("C5", "8n");
    testSynth.triggerAttackRelease("G5", "8n", "+0.2");
}

/**
 * This function is called reliably by setTimeout when the timer duration has elapsed.
 */
function onTimerEnd() {
    playSound();
    showSystemNotification();
    completionModal.classList.add('visible'); // Show in-page modal
    endSession(true); // End session automatically
}


// --- CORE TIMER LOGIC ---

/**
 * Updates the timer display element. This is run by requestAnimationFrame.
 */
function updateDisplayLoop() {
    if (!isRunning) return;

    // This calculation ensures the display is correct even after a background tab becomes active.
    const newTimeLeft = Math.round((targetTime - Date.now()) / 1000);
    if (newTimeLeft !== timeLeft) {
        timeLeft = newTimeLeft > 0 ? newTimeLeft : 0;
        updateDisplay();
    }

    if (timeLeft > 0) {
        animationFrameId = requestAnimationFrame(updateDisplayLoop);
    }
}

/**
 * Updates the text content of the timer display.
 */
function updateDisplay() {
    const minutes = Math.floor(timeLeft / 60);
    const seconds = timeLeft % 60;
    timerDisplay.textContent = `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
}

/**
 * Starts or pauses the timer, handling UI changes and session state.
 */
function startTimer() {
    // Start Tone.js & request notification permission on first user interaction.
    Tone.start();
    requestNotificationPermission();

    if (!isRunning) {
        // Prevent starting without a goal.
        if (!goalInput.value.trim()) {
            goalInput.classList.add('border-red-500', 'ring-red-500');
            goalInput.placeholder = "A goal is required to start!";
            setTimeout(() => {
                goalInput.classList.remove('border-red-500', 'ring-red-500');
                goalInput.placeholder = "Enter your goal for this session";
            }, 2000);
            return;
        }

        // --- Start the timer ---
        isRunning = true;
        targetTime = Date.now() + timeLeft * 1000;

        // This setTimeout is the key to background reliability.
        timeoutId = setTimeout(onTimerEnd, timeLeft * 1000);

        animationFrameId = requestAnimationFrame(updateDisplayLoop);

        startBtn.textContent = 'Pause';
        startBtn.classList.replace('bg-emerald-500', 'bg-orange-500');
        startBtn.classList.replace('hover:bg-emerald-600', 'hover:bg-orange-600');
        durationSelect.disabled = true;
        goalInput.disabled = true;

        if (!currentSession) {
            currentSession = {
                goal: goalInput.value,
                startTime: new Date().toISOString(),
                endTime: null, duration: 0, pauses: []
            };
        } else if (currentSession.pauses.length > 0) {
             const lastPause = currentSession.pauses[currentSession.pauses.length - 1];
             if (!lastPause.resumeTime) {
                lastPause.resumeTime = new Date().toISOString();
             }
        }

    } else {
        // --- Pause the timer ---
        isRunning = false;
        clearTimeout(timeoutId); // Stop the reliable end-timer.
        cancelAnimationFrame(animationFrameId); // Stop the visual updates.

        startBtn.textContent = 'Resume';
        startBtn.classList.replace('bg-orange-500', 'bg-emerald-500');
        startBtn.classList.replace('hover:bg-orange-600', 'hover:bg-emerald-600');

        currentSession.pauses.push({
            pauseTime: new Date().toISOString(),
            resumeTime: null
        });
    }
}

/**
 * Resets the timer to its initial state based on the selected duration.
 */
function resetTimer() {
    isRunning = false;
    clearTimeout(timeoutId);
    cancelAnimationFrame(animationFrameId);

    duration = parseInt(durationSelect.value) * 60;
    timeLeft = duration;
    updateDisplay();

    startBtn.textContent = 'Start';
    startBtn.classList.replace('bg-orange-500', 'bg-emerald-500');
    startBtn.classList.replace('hover:bg-orange-600', 'hover:bg-emerald-600');
    durationSelect.disabled = false;
    goalInput.disabled = false;
    goalInput.value = '';
    currentSession = null;
}

/**
 * Changes the timer duration when a new value is selected from the dropdown.
 */
function changeDuration() {
    if (!isRunning) {
        duration = parseInt(durationSelect.value) * 60;
        timeLeft = duration;
        updateDisplay();
    }
}

/**
 * Ends the current session, logs it, and resets the timer.
 * @param {boolean} isFinished - True if the session ended naturally by the timer finishing.
 */
function endSession(isFinished = false) {
     isRunning = false;
     clearTimeout(timeoutId);
     cancelAnimationFrame(animationFrameId);

     if (currentSession && !currentSession.endTime) { // Ensure session is only logged once
        currentSession.endTime = new Date().toISOString();

        let totalPauseTime = 0;
        currentSession.pauses.forEach(p => {
            if(p.resumeTime) {
                totalPauseTime += new Date(p.resumeTime) - new Date(p.pauseTime);
            }
        });

        currentSession.duration = Math.round(((new Date(currentSession.endTime) - new Date(currentSession.startTime)) - totalPauseTime) / 1000);

        sessions.unshift(currentSession);
        updateSessionLog();
        saveSessions();
     }

     resetTimer();
     if (isFinished) {
         goalInput.disabled = false;
     }
}

/**
 * Renders the session data into the log table.
 */
function updateSessionLog() {
    sessionLogBody.innerHTML = '';
    sessions.forEach((session, index) => {
        const row = document.createElement('tr');
        row.className = 'bg-white border-b';
        row.innerHTML = `
            <td class="px-6 py-4 font-medium text-gray-900 whitespace-nowrap">${session.goal}</td>
            <td class="px-6 py-4">${formatDate(session.startTime)}</td>
            <td class="px-6 py-4">${session.endTime ? formatDate(session.endTime) : 'In progress'}</td>
            <td class="px-6 py-4">${formatDuration(session.duration)}</td>
            <td class="px-6 py-4">${formatPauses(session.pauses)}</td>
            <td class="px-6 py-4">
                <button class="deleteBtn text-red-500 hover:text-red-700" data-index="${index}">❌</button>
            </td>
        `;
        sessionLogBody.appendChild(row);
    });
    jsonOutput.value = JSON.stringify(sessions, null, 2);

    document.querySelectorAll('.deleteBtn').forEach(btn => {
        btn.addEventListener('click', deleteSession);
    });
}

/**
 * Deletes a specific session from the log.
 * @param {Event} event - The click event from the delete button.
 */
function deleteSession(event) {
    const index = parseInt(event.currentTarget.getAttribute('data-index'));
    sessions.splice(index, 1);
    updateSessionLog();
    saveSessions();
}

/**
 * Clears all sessions from the log.
 */
function clearAllSessions() {
    if (confirm('Are you sure you want to delete all session logs?')) {
        sessions = [];
        updateSessionLog();
        saveSessions();
    }
}

// --- EVENT LISTENERS ---
startBtn.addEventListener('click', startTimer);
resetBtn.addEventListener('click', () => {
     if (currentSession) endSession();
     resetTimer();
});
endBtn.addEventListener('click', () => endSession());
durationSelect.addEventListener('change', changeDuration);
closeModalBtn.addEventListener('click', () => {
    completionModal.classList.remove('visible');
    // Stop the sound and dispose of the synth
    if (synth) {
        Tone.Transport.stop(); // Stop the transport which stops the loop
        Tone.Transport.cancel(); // Cancel all scheduled events
        synth.dispose();
        synth = null; // Clear the reference
    }
});
testSoundBtn.addEventListener('click', () => {
    Tone.start();
    playTestSound();
});
clearAllBtn.addEventListener('click', clearAllSessions);

// Keyboard shortcut for spacebar to start/pause.
document.addEventListener('keydown', (e) => {
    if (e.code === 'Space' && document.activeElement.tagName !== 'INPUT') {
        e.preventDefault();
        startTimer();
    }
});

// --- INITIALIZATION ---
loadSessions();
updateDisplay();
