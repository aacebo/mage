(function (root, factory) {
    const api = factory();
    if (typeof module === "object" && module.exports) module.exports = api;
    root.NeuronConsoleReducer = api;
})(typeof globalThis !== "undefined" ? globalThis : this, function () {
    "use strict";

    function createState(tenantId) {
        return {
            tenantId,
            eventIds: new Set(),
            events: [],
            actors: new Map(),
            chats: new Map(),
            traces: new Map(),
        };
    }

    function compareEvents(left, right) {
        return String(left.id).localeCompare(String(right.id));
    }

    function reduceEvent(state, event) {
        if (!event || event.tenant_id !== state.tenantId || state.eventIds.has(event.id)) return false;

        state.eventIds.add(event.id);
        insertSorted(state.events, event, compareEvents);

        let trace = state.traces.get(event.trace_id);
        if (!trace) {
            trace = { id: event.trace_id, events: [] };
            state.traces.set(event.trace_id, trace);
        }
        insertSorted(trace.events, event, compareEvents);

        const data = event.data || {};
        if ((event.key === "actor.create" || event.key === "actor.update") && data.actor) {
            state.actors.set(data.actor.id, data.actor);
        } else if (event.key === "chat.create" && data.chat) {
            const previous = state.chats.get(data.chat.id) || { members: [] };
            state.chats.set(data.chat.id, { ...previous, chat: data.chat });
        } else if (event.key === "chat.members" && data.chat_id) {
            const previous = state.chats.get(data.chat_id) || {};
            state.chats.set(data.chat_id, {
                ...previous,
                members: Array.from(new Set(data.actor_ids || [])),
            });
        }

        return true;
    }

    function reduceAll(state, events) {
        for (const event of [...events].sort(compareEvents)) reduceEvent(state, event);
        return state;
    }

    function insertSorted(values, value, compare) {
        let low = 0;
        let high = values.length;
        while (low < high) {
            const middle = (low + high) >>> 1;
            if (compare(values[middle], value) <= 0) low = middle + 1;
            else high = middle;
        }
        values.splice(low, 0, value);
    }

    function traces(state) {
        return Array.from(state.traces.values())
            .map((trace) => {
                const first = trace.events[0];
                const last = trace.events[trace.events.length - 1];
                return {
                    ...trace,
                    startedAt: first?.created_at || null,
                    updatedAt: last?.created_at || null,
                    keys: Array.from(new Set(trace.events.map((event) => event.key))),
                    agentIds: traceAgentIds(state, trace),
                };
            })
            .sort((left, right) => String(right.updatedAt).localeCompare(String(left.updatedAt)));
    }

    function traceAgentIds(state, trace) {
        const ids = new Set();
        const chatIds = new Set();

        for (const event of trace.events) {
            const data = event.data || {};
            if (data.actor?.role === "agent") ids.add(data.actor.id);
            if (data.message?.sent_by?.role === "agent") ids.add(data.message.sent_by.id);
            if (data.message?.created_by?.role === "agent") ids.add(data.message.created_by.id);
            if (data.chat?.id) chatIds.add(data.chat.id);
            if (data.chat_id) chatIds.add(data.chat_id);
            if (data.message?.chat?.id) chatIds.add(data.message.chat.id);
            if (Array.isArray(data.actor_ids)) {
                for (const id of data.actor_ids) {
                    if (state.actors.get(id)?.role === "agent") ids.add(id);
                }
            }
        }

        for (const chatId of chatIds) {
            for (const id of state.chats.get(chatId)?.members || []) {
                if (state.actors.get(id)?.role === "agent") ids.add(id);
            }
        }
        return Array.from(ids);
    }

    function eventChatId(event) {
        return (
            event?.data?.chat?.id ||
            event?.data?.chat_id ||
            event?.data?.message?.chat_id ||
            event?.data?.message?.chat?.id ||
            null
        );
    }

    function traceChatId(trace) {
        if (!trace) return null;
        for (const event of trace.events) {
            const chatId = eventChatId(event);
            if (chatId) return chatId;
        }
        return null;
    }

    function conversations(traceItems) {
        const grouped = new Map();
        for (const trace of traceItems) {
            if (!trace.keys.includes("message.inbound")) continue;
            const key = traceChatId(trace) || trace.id;
            if (!grouped.has(key)) grouped.set(key, trace);
        }
        return Array.from(grouped.values());
    }

    function conversationEvents(traceItems, chatId, fallbackTraceId) {
        const selected = chatId
            ? traceItems.filter((trace) => traceChatId(trace) === chatId)
            : traceItems.filter((trace) => trace.id === fallbackTraceId);
        return selected
            .flatMap((trace) => trace.events)
            .sort(compareEvents);
    }

    function route_agent_ids(state, event) {
        if (!event) return [];

        if (event.key === "chat.members") {
            return (event.data?.actor_ids || []).filter((id) => state.actors.get(id)?.role === "agent");
        }

        if (event.key === "message.create") {
            const chat_id = eventChatId(event);
            return (state.chats.get(chat_id)?.members || []).filter(
                (id) => state.actors.get(id)?.role === "agent",
            );
        }

        return [];
    }

    function topology(state) {
        const agents = Array.from(state.actors.values()).filter((actor) => actor.role === "agent");
        const agentIds = new Set(agents.map((agent) => agent.id));
        const broker_id = "broker_root";
        const elements = [
            {
                group: "nodes",
                data: {
                    id: broker_id,
                    kind: "broker",
                    label: "Broker cluster",
                    cluster: {
                        id: broker_id,
                        role: "broker",
                        name: "Broker cluster",
                        status: "online",
                        description: "Local routing cluster",
                    },
                },
                classes: "broker",
            },
        ];

        for (const agent of agents) {
            elements.push({
                group: "nodes",
                data: {
                    id: agent.id,
                    kind: "agent",
                    label: agent.name,
                    status: agent.status || "offline",
                    instances: agent.instances || 0,
                    actor: agent,
                },
                classes: `agent ${agent.status || "offline"}`,
            });
            elements.push({
                group: "edges",
                data: {
                    id: `routes_to:${broker_id}:${agent.id}`,
                    kind: "routes_to",
                    source: broker_id,
                    target: agent.id,
                    weight: 1,
                },
                classes: "routes_to",
            });
        }

        const coSelection = new Map();
        for (const chat of state.chats.values()) {
            const members = (chat.members || []).filter((id) => agentIds.has(id)).sort();
            for (let left = 0; left < members.length; left += 1) {
                for (let right = left + 1; right < members.length; right += 1) {
                    const key = `${members[left]}:${members[right]}`;
                    coSelection.set(key, (coSelection.get(key) || 0) + 1);
                }
            }
        }

        for (const [pair, weight] of coSelection) {
            const [source, target] = pair.split(":");
            elements.push({
                group: "edges",
                data: {
                    id: `co-selected:${pair}`,
                    kind: "co_selected",
                    source,
                    target,
                    weight,
                    label: weight > 1 ? String(weight) : "",
                },
                classes: "co-selected",
            });
        }

        return elements;
    }

    function latestCursor(state) {
        const event = state.events[state.events.length - 1];
        return event?.id || null;
    }

    function cursorAtOrAfter(cursor, target) {
        if (!target) return true;
        if (!cursor) return false;
        return cursor >= target;
    }

    return {
        compareEvents,
        conversationEvents,
        conversations,
        createState,
        cursorAtOrAfter,
        eventChatId,
        latestCursor,
        reduceAll,
        reduceEvent,
        route_agent_ids,
        topology,
        traceChatId,
        traceAgentIds,
        traces,
    };
});
