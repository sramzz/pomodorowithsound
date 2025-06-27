// Unit tests for Pomodoro Timer 2.0

// Mock Tone.js and Notification API for testing environment
const mockTone = {
    Synth: function() {
        return {
            toDestination: function() {
                return {
                    triggerAttackRelease: function() {},
                    dispose: function() {}
                };
            }
        };
    },
    Loop: function(callback, interval) {
        this.callback = callback;
        this.interval = interval;
        this.start = function() { this.started = true; return this; };
        this.stop = function() { this.started = false; };
        this.cancel = function() { this.started = false; };
        // Simulate loop execution for testing
        this.triggerLoop = function(time) { if(this.started) this.callback(time); };
        return this;
    },
    Transport: {
        start: function() { this.started = true; },
        stop: function() { this.started = false; },
        cancel: function() { this.scheduledEvents = []; },
        state: 'stopped',
        scheduledEvents: [] // Mock scheduled events
    },
    Time: function(val) { return val; }, // Mock Tone.Time
    now: function() { return performance.now() / 1000; }, // Mock Tone.now()
    context: {
        state: 'suspended',
        resume: function() { this.state = 'running'; return Promise.resolve(); }
    },
    start: function() { return Promise.resolve(); } // Mock Tone.start()
};

// Replace actual Tone with mock for tests
const Tone = mockTone;

// Mock Notification API
global.Notification = {
    permission: 'default',
    requestPermission: function() {
        return Promise.resolve('granted');
    },
    // Store notifications to check if they were created
    notifications: [],
    // Override the constructor
    __proto__: function(title, options) {
        const notification = { title, options };
        Notification.notifications.push(notification);
        return notification;
    }
};
// Reset notifications store for each test if needed
Notification.clearNotifications = () => { Notification.notifications = []; };


// --- Test Suite ---
console.log("Pomodoro Timer Tests Loaded. Run tests in browser console.");

function runTests() {
    console.log("--- Running Pomodoro Timer Tests ---");
    let testsPassed = 0;
    let testsFailed = 0;

    // Helper function to assert conditions
    function assert(condition, message) {
        if (condition) {
            console.log(`%cPASS: ${message}`, 'color: green');
            testsPassed++;
        } else {
            console.error(`%cFAIL: ${message}`, 'color: red');
            testsFailed++;
        }
    }

    // Helper to reset DOM and timer state before each test
    function resetTestEnvironment() {
        // Reset DOM elements (simplified)
        document.body.innerHTML = `
            <div id="timer">25:00</div>
            <button id="startBtn">Start</button>
            <button id="resetBtn">Reset</button>
            <button id="endBtn">End</button>
            <select id="durationSelect">
                <option value="1" selected>1 minute</option> <!-- Short duration for testing -->
            </select>
            <input type="text" id="goalInput" value="Test Goal">
            <div id="sessionLogBody"></div>
            <textarea id="jsonOutput"></textarea>
            <div id="completionModal" class="modal-overlay">
                 <button id="closeModalBtn">OK</button>
            </div>
        `;

        // Re-initialize necessary global variables from index.html script
        // These would be the variables that are not constants and are modified during runtime
        // Note: This is a simplification. In a more complex setup, you'd re-run the initialization logic
        // or have a dedicated function to reset state.

        // From index.html (ensure these are accessible or re-declared for tests)
        // This requires the script in index.html to be structured to allow re-initialization
        // or for these variables to be globally scoped (which is the case here).

        // Resetting state variables (mirroring those in index.html)
        targetTime = undefined;
        timeLeft = 1 * 60; // Default to 1 minute for tests
        isRunning = false;
        if (animationFrameId) cancelAnimationFrame(animationFrameId);
        animationFrameId = null;
        if (timeoutId) clearTimeout(timeoutId);
        timeoutId = null;
        duration = 1 * 60;
        currentSession = null;
        sessions = [];
        synth = null; // Reset synth instance

        // Re-attach event listeners or ensure they are active
        // This is tricky without reloading the script or having an init function.
        // For simplicity, we assume the main script's event listeners are still active
        // or we re-initialize them if the DOM was completely replaced.
        // If DOM is fully replaced, the original event listeners are gone.
        // A better approach is to have an initEventListeners() function in the main script.

        // Re-query DOM elements for the test functions
        // This is important if `document.body.innerHTML` was used.
        timerDisplay = document.getElementById('timer');
        startBtn = document.getElementById('startBtn');
        resetBtn = document.getElementById('resetBtn');
        endBtn = document.getElementById('endBtn');
        durationSelect = document.getElementById('durationSelect');
        goalInput = document.getElementById('goalInput');
        sessionLogBody = document.getElementById('sessionLogBody');
        jsonOutput = document.getElementById('jsonOutput');
        completionModal = document.getElementById('completionModal');
        closeModalBtn = document.getElementById('closeModalBtn');

        // Ensure the main script's event listeners are re-attached if DOM was cleared.
        // This would ideally be handled by an initialization function in the main script.
        // For now, we'll assume the main script's listeners are still somehow active or
        // that the functions we call directly will work.
        // A more robust way: call a function like `initializePomodoroApp()` if it exists.

        // Reset mocks
        Notification.clearNotifications();
        Tone.Transport.stop();
        Tone.Transport.cancel();
        Tone.Transport.state = 'stopped';
        Tone.Transport.scheduledEvents = [];
        if (synth) synth.dispose(); // Ensure synth is disposed if created in a test
    }

    // --- Test Cases ---

    // Test 1: Timer Initialization
    console.log("\n--- Test Set 1: Timer Initialization ---");
    resetTestEnvironment();
    assert(timerDisplay.textContent === "01:00", "Timer display initializes to selected duration (1 min for test).");
    assert(timeLeft === 60, "timeLeft variable initializes correctly.");
    assert(isRunning === false, "isRunning is initially false.");
    assert(goalInput.value === "Test Goal", "Goal input has test value.");

    // Test 2: Start Timer
    console.log("\n--- Test Set 2: Start Timer ---");
    resetTestEnvironment();
    startBtn.click(); // Simulate click
    assert(isRunning === true, "Timer starts and isRunning is true.");
    assert(startBtn.textContent === 'Pause', "Start button text changes to 'Pause'.");
    assert(goalInput.disabled === true, "Goal input is disabled after start.");
    assert(durationSelect.disabled === true, "Duration select is disabled after start.");
    assert(currentSession !== null, "currentSession is created.");
    assert(currentSession.goal === "Test Goal", "currentSession goal is correct.");

    // Test 3: Pause Timer
    console.log("\n--- Test Set 3: Pause Timer ---");
    resetTestEnvironment();
    startBtn.click(); // Start
    startBtn.click(); // Pause
    assert(isRunning === false, "Timer pauses and isRunning is false.");
    assert(startBtn.textContent === 'Resume', "Start button text changes to 'Resume'.");
    assert(currentSession.pauses.length === 1, "Pause is recorded in currentSession.");

    // Test 4: Resume Timer
    console.log("\n--- Test Set 4: Resume Timer ---");
    resetTestEnvironment();
    startBtn.click(); // Start
    startBtn.click(); // Pause
    startBtn.click(); // Resume
    assert(isRunning === true, "Timer resumes and isRunning is true.");
    assert(startBtn.textContent === 'Pause', "Start button text changes back to 'Pause'.");
    assert(currentSession.pauses.length === 1 && currentSession.pauses[0].resumeTime !== null, "Pause resume time is recorded.");

    // Test 5: Reset Timer
    console.log("\n--- Test Set 5: Reset Timer ---");
    resetTestEnvironment();
    startBtn.click(); // Start
    resetBtn.click(); // Reset
    assert(isRunning === false, "Timer is not running after reset.");
    assert(timeLeft === 60, "timeLeft resets to initial duration.");
    assert(timerDisplay.textContent === "01:00", "Timer display resets.");
    assert(startBtn.textContent === 'Start', "Start button text resets to 'Start'.");
    assert(goalInput.disabled === false, "Goal input is enabled after reset.");
    assert(durationSelect.disabled === false, "Duration select is enabled after reset.");
    assert(currentSession === null, "currentSession is cleared on reset.");


    // Test 6: Timer End and Notification (requires async handling)
    console.log("\n--- Test Set 6: Timer End and Sound/Notification ---");
    resetTestEnvironment();
    // Mock onTimerEnd to prevent actual timeout issues in test environment
    // We will call onTimerEnd manually to test its effects

    // Simulate timer running and then ending
    goalInput.value = "Timer End Test";
    startTimer(); // This sets up currentSession

    // Manually call onTimerEnd as if the timeout triggered
    onTimerEnd();

    assert(completionModal.classList.contains('visible'), "Completion modal is visible after timer ends.");
    assert(Notification.notifications.length > 0, "System notification was attempted.");
    if (Notification.notifications.length > 0) {
        assert(Notification.notifications[0].title === 'Pomodoro Complete!', "Notification title is correct.");
        assert(Notification.notifications[0].options.body.includes("Timer End Test"), "Notification body includes goal.");
    }
    assert(Tone.Transport.started === true, "Tone.Transport was started for sound loop.");
    assert(typeof synth !== 'undefined' && synth !== null, "Synth instance was created for sound.");
    // Note: Testing actual sound output is hard in unit tests. We test if the sound machinery was invoked.


    // Test 7: Closing Modal Stops Sound
    console.log("\n--- Test Set 7: Closing Modal Stops Sound ---");
    resetTestEnvironment();
    goalInput.value = "Modal Sound Test";
    startTimer();
    onTimerEnd(); // Manually trigger end: shows modal, starts sound

    assert(Tone.Transport.started === true, "Sound transport is active before modal close.");
    assert(synth !== null, "Synth instance exists before modal close.");

    closeModalBtn.click(); // Simulate closing the modal

    assert(!completionModal.classList.contains('visible'), "Completion modal is hidden after close.");
    assert(Tone.Transport.started === false, "Sound transport is stopped after modal close.");
    assert(synth === null, "Synth instance is disposed and cleared after modal close.");


    // Test 8: Session Logging
    console.log("\n--- Test Set 8: Session Logging ---");
    resetTestEnvironment();
    goalInput.value = "Logging Test";
    startBtn.click(); // Start
    // Simulate time passing for the session to have a duration
    // In a real test, you might advance a mock clock. Here, we'll just end it.
    endSession(true); // End session as if finished

    assert(sessions.length === 1, "Session is added to the sessions array.");
    if (sessions.length === 1) {
        assert(sessions[0].goal === "Logging Test", "Logged session goal is correct.");
        assert(sessions[0].endTime !== null, "Logged session has an end time.");
        // Duration check might be tricky if time doesn't actually pass in test.
        // onTimerEnd which calls endSession(true) will use the original duration.
        assert(sessions[0].duration > 0, "Logged session has a duration.");
    }
    assert(sessionLogBody.children.length === 1, "Session log table is updated in DOM.");
    assert(jsonOutput.value.length > 0, "JSON output is generated.");


    // Test 9: Goal Input Validation
    console.log("\n--- Test Set 9: Goal Input Validation ---");
    resetTestEnvironment();
    goalInput.value = ""; // Empty goal
    startBtn.click();
    assert(isRunning === false, "Timer does not start with an empty goal.");
    assert(goalInput.classList.contains('border-red-500'), "Goal input shows validation error style.");
    // We'd need a way to test the placeholder change and its reset, possibly with timers.
    // For now, checking the class is a good start.

    // --- Test Summary ---
    console.log("\n--- Test Summary ---");
    console.log(`${testsPassed} tests passed.`);
    if (testsFailed > 0) {
        console.error(`${testsFailed} tests FAILED.`);
    } else {
        console.log("All tests passed successfully!");
    }
    console.log("--------------------------------------");

    return testsFailed === 0;
}

// To run tests, open index.html in a browser, open the console, and type: runTests()
// Make sure this script (tests.js) is included in index.html AFTER the main script.
// e.g., <script src="tests.js"></script> at the end of the body.

// Example of how you might include it in index.html:
// ... (rest of your HTML)
// <script> /* main pomodoro script */ </script>
// <script src="tests.js"></script>
// </body>
// </html>
// Then, in the browser's developer console, type `runTests()` and press Enter.
