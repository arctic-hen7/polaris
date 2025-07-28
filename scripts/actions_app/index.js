// Parse the contexts, people, and raw tasks
const [CONTEXTS, PEOPLE, tasksRaw] = JSON.parse(
    document.getElementById("actionsData").textContent.trim(),
);
// Expand the tasks so they're easier to work with
const tasks = [];
for (const task of tasksRaw) {
    tasks.push({
        "id": task[0],
        "title": task[1],
        "body": task[2],
        "scheduled": task[3] ? new Date(task[3]) : null,
        "deadline": task[4] ? new Date(task[4]) : null,
        "priority": task[5],
        "effort": task[6],
        "contexts": task[7],
        "people": task[8],
    });
}

// These are used for translating numerical priorities and efforts into human-readable strings
const PRIORITIES = {
    0: "low",
    1: "medium",
    2: "high",
    3: "important",
};
const EFFORTS = {
    0: "minimal",
    1: "low",
    2: "medium",
    3: "high",
    4: "total",
};

const setPanelTasks = () => {
    document.getElementById("panelContexts").classList.add("hidden");
    document.getElementById("panelTasks").classList.remove("hidden");
    document.getElementById("tasksButton").classList.add("bg-sky-300/10");
    document.getElementById("contextsButton").classList.remove("bg-sky-300/10");
};
const setPanelContexts = () => {
    document.getElementById("panelTasks").classList.add("hidden");
    document.getElementById("panelContexts").classList.remove("hidden");
    document.getElementById("contextsButton").classList.add("bg-sky-300/10");
    document.getElementById("tasksButton").classList.remove("bg-sky-300/10");
};

// Computes the contexts the user needs to enter on the given date, and the tasks they
// need to complete within those contexts. This is the main way urgent actions are
// shown. These are then displayed in the target contexts panel.
//
// The given date should be without a timezone.
const displayTargetContexts = (date, currentDate) => {
    date.setHours(23, 59, 59, 999);

    const targetContexts = {};
    for (const task of tasks) {
        if (task.scheduled && task.scheduled > date) {
            continue;
        }
        if (task.deadline && task.deadline <= date) {
            // If this task has multiple contexts, add it to all those contexts
            for (const ctx of task.contexts) {
                if (!targetContexts[ctx]) {
                    targetContexts[ctx] = [];
                }
                targetContexts[ctx].push(task);
            }
            if (task.contexts.length === 0) {
                // If this task has no contexts, add it to the "No Context" context
                if (!targetContexts[-1]) {
                    targetContexts[-1] = [];
                }
                targetContexts[-1].push(task);
            }
        }
    }

    document.getElementById("urgentContexts").innerHTML = "";
    for (const ctxIdx in targetContexts) {
        let html = `<h3 class="text-xl p-1 underline w-full bg-sky-300/20">${
            ctxIdx === -1
                ? "No Context"
                : CONTEXTS[ctxIdx].charAt(0).toUpperCase() +
                    CONTEXTS[ctxIdx].slice(1)
        }</h3>`;
        for (const task of targetContexts[ctxIdx]) {
            html += displayTask(task, currentDate);
        }
        document.getElementById("urgentContexts").innerHTML += html;
    }
};

// Converts the given task object into an HTML string.
const displayTask = (task, currentDate) => {
    const contexts = [];
    for (const ctxIdx of task.contexts) {
        contexts.push(CONTEXTS[ctxIdx]);
    }
    const contextsStr = contexts.length === 0 ? "none" : contexts.join(", ");

    // Can assemble most off the bat, but some will need to be substituted in
    let htmlStr = `<pre class="text-sm my-3 whitespace-pre-wrap">
  <strong>→ ${task.title}</strong>%SCHEDULED%%DEADLINE%
  <i>Priority: <strong class="text-green-600">${
        PRIORITIES[task.priority]
    }</strong></i>
  <i>Contexts: <strong class="text-blue-600">${contextsStr}</strong></i>
  <i>Effort: <strong class="text-sky-500">${
        EFFORTS[task.effort]
    }</strong></i>%PEOPLE_NEEDED%%BODY%
</pre>`;
    htmlStr = htmlStr.replace(
        "%SCHEDULED%",
        task.scheduled
            ? `\n  <i>Scheduled: <strong class="text-amber-600">${
                formatDate(task.scheduled, currentDate)
            }</strong></i>`
            : "",
    );
    htmlStr = htmlStr.replace(
        "%DEADLINE%",
        task.deadline
            ? `\n  <i>Deadline: <strong class="text-red-500">${
                formatDate(task.deadline, currentDate)
            }</strong></i>`
            : "",
    );
    let peopleHtml = "";
    for (const personIdx of task.people) {
        peopleHtml += `\n    <i>- <strong>${PEOPLE[personIdx]}</strong></i>`;
    }
    htmlStr = htmlStr.replace(
        "%PEOPLE_NEEDED%",
        task.people.length !== 0
            ? `\n  <i>People needed:</i>${peopleHtml}`
            : "",
    );
    htmlStr = htmlStr.replace(
        "%BODY%",
        task.body
            ? `\n\n  <span class="break-all">${
                task.body.replace("\n", "\n  ")
            }</span>`
            : "",
    );

    return htmlStr;
};

// Formats dates for human readability relative to a current date.
const formatDate = (dateTime, currentDate) => {
    const date = new Date(dateTime);
    date.setHours(0, 0, 0, 0);
    currentDate.setHours(0, 0, 0, 0);
    const timeStr = dateTime.toLocaleTimeString(undefined, {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
    });

    // Need to force this into the local timezone
    const weekdays = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];

    const daysDifference = Math.floor(
        (date - currentDate) / (1000 * 60 * 60 * 24),
    );

    let dayStr = "";
    if (daysDifference === 0) {
        dayStr = "today";
    } else if (daysDifference === 1) {
        dayStr = "tomorrow";
    } else if (daysDifference === -1) {
        dayStr = "yesterday";
    } else if (2 <= daysDifference && daysDifference < 7) {
        dayStr = `${weekdays[date.getDay()]}`;
    } else if (-7 < daysDifference && daysDifference < 0) {
        dayStr = `last ${weekdays[date.getDay()]}`;
    } else if (0 < daysDifference && daysDifference < 14) {
        dayStr = `next ${weekdays[date.getDay()]}`;
    } else {
        dayStr = `${weekdays[date.getDay()]} ${
            dateTime.toLocaleDateString("en-CA")
        }`;
    }

    if (timeStr && timeStr !== "23:59:59" && timeStr !== "00:00:00") {
        dayStr += ` at ${timeStr.slice(0, 5)}`; // Only show hours and minutes
    }

    return dayStr;
};

// Filters the tasks down by the given contexts, people, and maximum effort.
const filter = (contextsArr, peopleArr, maxEffort) => {
    const contexts = contextsArr
        ? new Set(contextsArr.map((ctx) => CONTEXTS.indexOf(ctx)))
        : null;
    const people = peopleArr
        ? new Set(peopleArr.map((person) => PEOPLE.indexOf(person)))
        : null;

    const filtered = [];
    for (const task of tasks) {
        if (contexts) {
            let weHaveAll = true;
            let itemHasOne = false;
            for (const ctxIdx of task.contexts) {
                if (!contexts.has(ctxIdx)) {
                    weHaveAll = false;
                    break;
                } else {
                    itemHasOne = true;
                }
            }
            if (!weHaveAll || !itemHasOne) {
                continue;
            }
        }
        if (people) {
            let weHaveAll = true;
            let itemHasOne = false;
            for (const personIdx of task.people) {
                if (!people.has(personIdx)) {
                    weHaveAll = false;
                    break;
                } else {
                    itemHasOne = true;
                }
            }
            if (!weHaveAll || !itemHasOne) {
                continue;
            }
        }
        if (task.effort > maxEffort) {
            continue;
        }

        filtered.push(task);
    }

    return filtered;
};

// Function called from HTML that runs the filter and displays results.
const doFilter = (currentDate) => {
    currentDate.setHours(23, 59, 59, 999);
    // `<select>`, so guaranteed to be right
    const maxEffort = parseInt(document.getElementById("effort").value);
    const contexts = Array.from(
        document.getElementById("contextsSelect").selectedOptions,
    ).map((option) => option.value);
    const people = Array.from(
        document.getElementById("peopleSelect").selectedOptions,
    ).map((option) => option.value);

    const filtered = filter(
        contexts.length === 0 ? null : contexts,
        people.length === 0 ? null : people,
        maxEffort,
    );

    document.getElementById("tasks").innerHTML = "";
    for (const task of filtered) {
        if (task.scheduled && task.scheduled > currentDate) {
            continue;
        }
        document.getElementById("tasks").innerHTML += displayTask(
            task,
            currentDate,
        );
    }
};

// Display the target contexts in their panel
displayTargetContexts(new Date(), new Date());

// Populate the context/people dropdowns with the right options
const contextSelect = document.getElementById("contextsSelect");
const sortedContexts = CONTEXTS.slice().sort((a, b) => a.localeCompare(b));
for (const ctx of sortedContexts) {
    const option = document.createElement("option");
    option.value = ctx;
    option.innerText = ctx
        .split("_") // Split the string at underscores
        .map((word) =>
            word.charAt(0).toUpperCase() + // Capitalize first letter
            word.slice(1) // Add the rest of the word in lowercase (if needed)
        )
        .join(" ");
    contextSelect.appendChild(option);
}
const peopleSelect = document.getElementById("peopleSelect");
const sortedPeople = PEOPLE.slice().sort((a, b) => a.localeCompare(b));
for (const person of sortedPeople) {
    const option = document.createElement("option");
    option.value = person;
    option.innerText = person;
    peopleSelect.appendChild(option);
}

// Do an empty filter to show all tasks, ordered by urgency
doFilter(new Date());
