(function () {
    "use strict";

    const Reducer = window.MageConsoleReducer;

    function readConfig() {
        return JSON.parse(document.getElementById("console-config").textContent);
    }

    function websocketUrl(path, params) {
        const url = new URL(path, window.location.href);
        url.protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        for (const [key, value] of Object.entries(params || {})) {
            if (value !== null && value !== undefined && value !== "") url.searchParams.set(key, value);
        }
        return url.toString();
    }

    async function openDatabase(config) {
        const name = `mage-console-${config.reducer_version}-${config.tenant_id}`;
        const activeKey = "mage-console-active-cache";
        const previous = localStorage.getItem(activeKey);
        if (previous && previous !== name) {
            await new Promise((resolve) => {
                const request = indexedDB.deleteDatabase(previous);
                request.onsuccess = resolve;
                request.onerror = resolve;
                request.onblocked = resolve;
            });
        }
        localStorage.setItem(activeKey, name);

        return new Promise((resolve, reject) => {
            const request = indexedDB.open(name, 1);
            request.onupgradeneeded = () => {
                const database = request.result;
                if (!database.objectStoreNames.contains("events")) {
                    database.createObjectStore("events", {
                        keyPath: ["tenant_id", "created_at", "id"],
                    });
                }
            };
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
    }

    function databaseRequest(database, mode, operation) {
        return new Promise((resolve, reject) => {
            const transaction = database.transaction("events", mode);
            const request = operation(transaction.objectStore("events"));
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
    }

    async function decodeWebSocketJson(data) {
        if (typeof data === "string") return JSON.parse(data);
        if (data instanceof Blob) return JSON.parse(await data.text());
        if (data instanceof ArrayBuffer) return JSON.parse(new TextDecoder().decode(data));
        if (ArrayBuffer.isView(data)) return JSON.parse(new TextDecoder().decode(data));
        throw new Error("Unsupported WebSocket frame type");
    }

    function createConsole() {
        const config = readConfig();
        const state = Reducer.createState(config.tenant_id);

        return {
            config,
            state,
            database: null,
            eventSocket: null,
            agentSocket: null,
            agentConnectRequestId: null,
            graph: null,
            reconnectTimer: null,
            reconnectDelay: 800,
            route_pulses: new Map(),
            recent_route_traces: new Map(),
            route_animation_frame: null,

            streamStatus: "starting",
            agentStatus: "disconnected",
            tab: "compose",
            senderMode: "user",
            agents: [],
            traceItems: [],
            selectedTraceId: null,
            activeChatId: null,
            selectedEvent: null,
            selectedEntity: null,
            pendingTraceId: null,
            draftTraceId: crypto.randomUUID(),
            error: "",
            notice: "",

            userExternalId: "console-user",
            userName: "Console User",
            selectedAgentId: "",
            agentSecret: "",
            subject: "",
            messageText: "",
            chatDraft: "",
            contentBlocks: [],
            metadataText: "{}",
            rawMode: false,
            rawPayload: "",
            traceFilter: "",
            keyFilter: "",
            traceAgentFilter: "",
            traceAfter: "",
            traceBefore: "",
            inspectorMode: "tree",
            inspectorSearch: "",
            expandedJsonPaths: ["$", "data"],

            async init() {
                try {
                    this.database = await openDatabase(config);
                    const cached = await databaseRequest(this.database, "readonly", (store) => store.getAll());
                    Reducer.reduceAll(state, cached);
                    this.refreshDerived();
                    this.openEventStream();
                } catch (error) {
                    this.error = `Local event cache unavailable: ${error.message}`;
                    this.openEventStream();
                }
                window.addEventListener("beforeunload", () => {
                    this.eventSocket?.close();
                    this.agentSocket?.close();
                    if (this.route_animation_frame !== null) {
                        cancelAnimationFrame(this.route_animation_frame);
                    }
                });
            },

            openEventStream() {
                clearTimeout(this.reconnectTimer);
                const cursor = Reducer.latestCursor(state);
                const url = websocketUrl("/console/connect", {
                    after_id: cursor,
                });
                this.streamStatus = "connecting";
                const socket = new WebSocket(url);
                this.eventSocket = socket;

                socket.onopen = () => {
                    this.reconnectDelay = 800;
                    this.streamStatus = Reducer.cursorAtOrAfter(cursor, config.high_water_cursor)
                        ? "live"
                        : "replaying";
                };
                socket.onmessage = (message) => {
                    try {
                        const event = JSON.parse(message.data);
                        this.ingestEvent(event);
                        if (Reducer.cursorAtOrAfter(Reducer.latestCursor(state), config.high_water_cursor)) {
                            this.streamStatus = "live";
                        }
                    } catch (error) {
                        this.error = `Invalid event frame: ${error.message}`;
                    }
                };
                socket.onerror = () => {
                    this.streamStatus = "degraded";
                };
                socket.onclose = () => {
                    if (this.eventSocket !== socket) return;
                    this.streamStatus = "offline";
                    this.reconnectTimer = setTimeout(() => this.openEventStream(), this.reconnectDelay);
                    this.reconnectDelay = Math.min(this.reconnectDelay * 1.8, 8000);
                };
            },

            ingestEvent(event) {
                if (!Reducer.reduceEvent(state, event)) return;
                if (this.database) {
                    databaseRequest(this.database, "readwrite", (store) => store.put(event)).catch(() => {});
                }
                if (this.pendingTraceId === event.trace_id) {
                    this.selectedTraceId = event.trace_id;
                    this.selectedEvent = event;
                    this.pendingTraceId = null;
                    this.tab = "chat";
                }
                if (this.selectedTraceId === event.trace_id) {
                    this.activeChatId = this.eventChatId(event) || this.activeChatId;
                }
                this.refreshDerived(event);
                if (this.streamStatus === "live") this.capture_route(event);
            },

            refreshDerived(event = null) {
                this.agents = Array.from(state.actors.values())
                    .filter((actor) => actor.role === "agent")
                    .sort((left, right) => left.name.localeCompare(right.name));
                this.traceItems = Reducer.traces(state);
                if (!this.selectedAgentId && this.agents.length) this.selectedAgentId = this.agents[0].id;
                if (this.selectedTraceId && !state.traces.has(this.selectedTraceId)) this.selectedTraceId = null;
                if (!this.graph) return;

                if (!event || ["actor.create", "actor.update", "chat.members"].includes(event.key)) {
                    queueMicrotask(() => this.renderGraph());
                } else {
                    queueMicrotask(() => this.applyTraceHighlight());
                }
            },

            selectTab(tab) {
                this.tab = tab;
                if (tab === "topology") {
                    setTimeout(() => {
                        this.ensureGraph();
                        requestAnimationFrame(() => this.fitGraph());
                    }, 0);
                }
            },

            ensureGraph() {
                if (this.graph || !this.$refs.topology || !window.cytoscape) return;
                this.graph = window.cytoscape({
                    container: this.$refs.topology,
                    elements: [],
                    minZoom: 0.35,
                    maxZoom: 2.4,
                    style: [
                        {
                            selector: "node",
                            style: {
                                "font-family": "Inter, ui-sans-serif, system-ui",
                                "font-size": 11,
                                "text-valign": "bottom",
                                "text-margin-y": 8,
                                "text-wrap": "wrap",
                                "text-max-width": 112,
                                color: "#a9b2c3",
                                label: "data(label)",
                                "background-color": "#141922",
                                "border-width": 1,
                                "border-color": "#394253",
                            },
                        },
                        {
                            selector: "node.agent",
                            style: {
                                width: 46,
                                height: 46,
                                "background-color": "#111722",
                                "border-width": 2,
                                "border-color": "#596579",
                            },
                        },
                        {
                            selector: "node.agent.online",
                            style: {
                                "border-color": "#34d399",
                            },
                        },
                        {
                            selector: "node.broker",
                            style: {
                                width: 58,
                                height: 38,
                                shape: "round-rectangle",
                                "background-color": "#102027",
                                "border-width": 2,
                                "border-color": "#67e8f9",
                                color: "#d9f8fc",
                                "font-weight": 600,
                            },
                        },
                        {
                            selector: "edge",
                            style: {
                                width: 1,
                                "line-color": "#2b3442",
                                "curve-style": "bezier",
                                opacity: 0.7,
                            },
                        },
                        {
                            selector: "edge.routes_to",
                            style: {
                                width: 1.5,
                                "line-color": "#35515e",
                                "target-arrow-color": "#35515e",
                                "target-arrow-shape": "triangle",
                                "arrow-scale": 0.7,
                                opacity: 0.85,
                            },
                        },
                        {
                            selector: "edge.co-selected",
                            style: {
                                width: "mapData(weight, 1, 8, 1.5, 6)",
                                "line-color": "#557087",
                                label: "data(label)",
                                "font-size": 9,
                                color: "#68768a",
                            },
                        },
                        {
                            selector: "node.trace-match",
                            style: {
                                "border-color": "#67e8f9",
                                "border-width": 3,
                            },
                        },
                        {
                            selector: "edge.trace-match",
                            style: {
                                width: 2.5,
                                "line-color": "#67e8f9",
                                "target-arrow-color": "#67e8f9",
                                opacity: 0.95,
                            },
                        },
                        {
                            selector: "node.route_path",
                            style: {
                                "border-color": "#a5f3fc",
                                "border-width": "mapData(route_intensity, 0, 1, 2, 6)",
                                "overlay-color": "#22d3ee",
                                "overlay-opacity": "mapData(route_intensity, 0, 1, 0, 0.2)",
                                "overlay-padding": "mapData(route_intensity, 0, 1, 0, 14)",
                            },
                        },
                        {
                            selector: "edge.route_path",
                            style: {
                                width: "mapData(route_intensity, 0, 1, 1.5, 5)",
                                "line-color": "#67e8f9",
                                "target-arrow-color": "#a5f3fc",
                                opacity: "mapData(route_intensity, 0, 1, 0.72, 1)",
                                "underlay-color": "#22d3ee",
                                "underlay-opacity": "mapData(route_intensity, 0, 1, 0, 0.28)",
                                "underlay-padding": "mapData(route_intensity, 0, 1, 0, 5)",
                            },
                        },
                        {
                            selector: ":selected",
                            style: {
                                "border-color": "#f8fafc",
                                "border-width": 2,
                            },
                        },
                    ],
                });
                this.graph.on("tap", "node, edge", (event) => {
                    const data = event.target.data();
                    this.selectedEntity = data.actor || data.cluster || {
                        kind: data.kind,
                        weight: data.weight,
                        source: data.source,
                        target: data.target,
                    };
                });
                this.renderGraph();
            },

            renderGraph() {
                if (!this.graph) return;

                const previous_positions = new Map();
                this.graph.nodes().forEach((node) => previous_positions.set(node.id(), node.position()));
                const previous_pan = this.graph.pan();
                const previous_zoom = this.graph.zoom();
                const elements = Reducer.topology(state);
                const next_node_ids = new Set(
                    elements.filter((element) => element.group === "nodes").map((element) => element.data.id),
                );
                const needs_layout =
                    previous_positions.size === 0 ||
                    previous_positions.size !== next_node_ids.size ||
                    Array.from(next_node_ids).some((id) => !previous_positions.has(id));

                this.graph.elements().remove();
                this.graph.add(elements);

                if (!needs_layout) {
                    this.graph.nodes().forEach((node) => node.position(previous_positions.get(node.id())));
                }

                this.applyTraceHighlight();
                this.animate_route_pulses();

                if (!needs_layout) {
                    this.graph.zoom(previous_zoom);
                    this.graph.pan(previous_pan);
                    return;
                }

                const broker = this.graph.getElementById("broker_root");
                this.graph
                    .layout({
                        name: "concentric",
                        animate: false,
                        fit: false,
                        padding: 80,
                        startAngle: -Math.PI / 2,
                        sweep: 2 * Math.PI,
                        clockwise: true,
                        equidistant: true,
                        minNodeSpacing: 70,
                        concentric: (node) => {
                            if (node.data("kind") === "broker") return 3;
                            return 2;
                        },
                        levelWidth: () => 1,
                    })
                    .run();
                broker.position({
                    x: this.$refs.topology.clientWidth / 2,
                    y: this.$refs.topology.clientHeight / 2,
                });
                broker.lock();

                const layout = this.graph.layout({
                    name: "cose",
                    animate: false,
                    randomize: false,
                    fit: false,
                    padding: 80,
                    nodeRepulsion: 90000,
                    nodeOverlap: 24,
                    idealEdgeLength: 135,
                    edgeElasticity: 110,
                    nestingFactor: 1.2,
                    gravity: 0.4,
                    numIter: 1400,
                    initialTemp: 220,
                    coolingFactor: 0.95,
                    minTemp: 1,
                    componentSpacing: 100,
                });
                layout.on("layoutstop", () => {
                    broker.unlock();
                    requestAnimationFrame(() => {
                        this.fitGraph();
                        this.animate_route_pulses();
                    });
                });
                layout.run();
            },

            fitGraph() {
                if (!this.graph) return;
                this.graph.resize();

                const broker = this.graph.getElementById("broker_root");
                if (!broker.length) {
                    this.graph.fit(undefined, 60);
                    return;
                }

                const bounds = this.graph.nodes().boundingBox({
                    includeNodes: true,
                    includeEdges: false,
                    includeLabels: true,
                });
                const broker_position = broker.position();
                const horizontal_extent = Math.max(
                    broker_position.x - bounds.x1,
                    bounds.x2 - broker_position.x,
                    1,
                );
                const vertical_extent = Math.max(
                    broker_position.y - bounds.y1,
                    bounds.y2 - broker_position.y,
                    1,
                );
                const available_width = Math.max(1, this.$refs.topology.clientWidth - 120);
                const available_height = Math.max(1, this.$refs.topology.clientHeight - 120);
                const zoom = Math.max(
                    0.35,
                    Math.min(2.4, available_width / (horizontal_extent * 2), available_height / (vertical_extent * 2)),
                );

                this.graph.zoom(zoom);
                this.graph.pan({
                    x: this.$refs.topology.clientWidth / 2 - broker_position.x * zoom,
                    y: this.$refs.topology.clientHeight / 2 - broker_position.y * zoom,
                });
            },

            applyTraceHighlight() {
                if (!this.graph) return;
                this.graph.elements().removeClass("trace-match");
                const trace = this.traceItems.find((item) => item.id === this.selectedTraceId);
                const agent_ids = trace?.agentIds || [];
                if (!agent_ids.length) return;

                this.graph.getElementById("broker_root").addClass("trace-match");
                for (const id of agent_ids) {
                    this.graph.getElementById(id).addClass("trace-match");
                    this.graph.getElementById(`routes_to:broker_root:${id}`).addClass("trace-match");
                }
            },

            capture_route(event) {
                const agent_ids = Reducer.route_agent_ids(state, event);
                if (!agent_ids.length) return;

                const now = Date.now();
                for (const [trace_id, expires_at] of this.recent_route_traces) {
                    if (expires_at <= now) this.recent_route_traces.delete(trace_id);
                }
                if (this.recent_route_traces.has(event.trace_id)) return;
                this.recent_route_traces.set(event.trace_id, now + 60000);

                const reduced_motion = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
                const duration = reduced_motion ? 350 : 4200;
                for (const agent_id of agent_ids) {
                    this.route_pulses.set(agent_id, {
                        started_at: now,
                        expires_at: now + duration,
                        duration,
                    });
                }
                this.animate_route_pulses();
            },

            animate_route_pulses() {
                if (this.route_animation_frame !== null || this.route_pulses.size === 0) return;

                const tick = () => {
                    this.route_animation_frame = null;
                    const now = Date.now();
                    let broker_intensity = 0;

                    for (const [agent_id, pulse] of this.route_pulses) {
                        const node = this.graph?.getElementById(agent_id);
                        const edge = this.graph?.getElementById(`routes_to:broker_root:${agent_id}`);

                        if (now >= pulse.expires_at) {
                            this.route_pulses.delete(agent_id);
                            node?.removeClass("route_path");
                            edge?.removeClass("route_path");
                            continue;
                        }

                        const elapsed = now - pulse.started_at;
                        const attack = Math.min(1, elapsed / Math.min(180, pulse.duration / 2));
                        const fade = Math.max(
                            0,
                            1 - Math.max(0, elapsed - 180) / Math.max(1, pulse.duration - 180),
                        );
                        const intensity = attack * Math.pow(fade, 1.6);
                        broker_intensity = Math.max(broker_intensity, intensity);

                        if (node?.length) {
                            node.data("route_intensity", intensity);
                            node.addClass("route_path");
                        }
                        if (edge?.length) {
                            edge.data("route_intensity", intensity);
                            edge.addClass("route_path");
                        }
                    }

                    const broker = this.graph?.getElementById("broker_root");
                    if (broker_intensity > 0 && broker?.length) {
                        broker.data("route_intensity", broker_intensity);
                        broker.addClass("route_path");
                    } else {
                        broker?.removeClass("route_path");
                    }

                    if (this.route_pulses.size > 0) {
                        this.route_animation_frame = requestAnimationFrame(tick);
                    } else {
                        this.graph?.elements(".route_path").removeClass("route_path");
                    }
                };

                this.route_animation_frame = requestAnimationFrame(tick);
            },

            selectAgent(agentId) {
                if (this.agentSocket) this.disconnectAgent();
                this.selectedAgentId = agentId;
                this.agentSecret = sessionStorage.getItem(`mage-agent-secret:${agentId}`) || "";
                this.selectedEntity = this.agents.find((agent) => agent.id === agentId) || null;
            },

            connectAgent() {
                this.error = "";
                const agent = this.agents.find((item) => item.id === this.selectedAgentId);
                if (!agent || !this.agentSecret) {
                    this.error = "Select an agent and enter its secret.";
                    return;
                }
                this.disconnectAgent();
                sessionStorage.setItem(`mage-agent-secret:${agent.id}`, this.agentSecret);
                this.agentStatus = "connecting";
                const socket = new WebSocket(websocketUrl("/agents/connect"));
                const connectRequestId = crypto.randomUUID();
                this.agentConnectRequestId = connectRequestId;
                this.agentSocket = socket;
                socket.binaryType = "arraybuffer";
                socket.onopen = () => {
                    socket.send(
                        JSON.stringify({
                            id: connectRequestId,
                            method: "connect",
                            params: {
                                id: agent.id,
                                name: agent.name,
                                description: agent.description || "",
                                secret: this.agentSecret,
                                skills: agent.skills || [],
                            },
                        }),
                    );
                };
                socket.onmessage = async (message) => {
                    try {
                        const frame = await decodeWebSocketJson(message.data);
                        if (!frame || typeof frame !== "object") throw new Error("Invalid ATP frame");
                        if (!frame.id) {
                            if (!frame.name || !("body" in frame)) throw new Error("Invalid ATP frame");
                            this.notice = `Received · ${frame.name}`;
                            return;
                        }

                        if (frame.error) {
                            const detail = frame.error.message || `ATP error ${frame.error.code}`;
                            if (frame.id === connectRequestId) {
                                this.agentStatus = "error";
                                this.agentConnectRequestId = null;
                                socket.close(1008, "connect rejected");
                            }
                            if (this.pendingTraceId === frame.id) this.pendingTraceId = null;
                            this.error = detail;
                            return;
                        }

                        if (frame.id === connectRequestId) {
                            this.agentConnectRequestId = null;
                            this.agentStatus = "connected";
                            this.notice = `${agent.name} connected`;
                        } else if (frame.id === this.pendingTraceId) {
                            this.pendingTraceId = null;
                            this.notice = `Accepted · ${frame.id.slice(0, 8)}`;
                        }
                    } catch (error) {
                        this.error = `Invalid ATP response: ${error.message}`;
                    }
                };
                socket.onclose = (event) => {
                    if (this.agentSocket !== socket) return;
                    this.agentSocket = null;
                    this.agentConnectRequestId = null;
                    this.agentStatus = "disconnected";
                    if (event.code !== 1000 && event.reason) this.error = event.reason;
                };
                socket.onerror = () => {
                    this.agentStatus = "error";
                };
            },

            disconnectAgent() {
                const socket = this.agentSocket;
                this.agentSocket = null;
                this.agentConnectRequestId = null;
                if (socket && socket.readyState < WebSocket.CLOSING) socket.close(1000, "console disconnect");
                this.agentStatus = "disconnected";
            },

            structuredPayload() {
                const metadata = this.parseMetadata();
                const content = this.structuredContent();
                if (this.senderMode === "agent") {
                    return {
                        chat_id: this.currentChatId(),
                        content,
                        metadata,
                    };
                }
                return {
                    subject: this.subject || null,
                    content,
                    metadata,
                    from: {
                        id: this.userExternalId,
                        name: this.userName,
                    },
                };
            },

            structuredContent() {
                const content = [];
                if (this.messageText.trim()) content.push({ type: "text", text: this.messageText });
                for (const block of this.contentBlocks) {
                    if (block.type === "text") {
                        if (block.value.trim()) content.push({ type: "text", text: block.value });
                    } else if (block.type === "json") {
                        content.push({ type: "json", json: JSON.parse(block.value) });
                    } else if (block.type === "file") {
                        const uri = new URL(block.value).toString();
                        content.push({
                            type: "file",
                            ...(block.name.trim() ? { name: block.name.trim() } : {}),
                            uri,
                        });
                    }
                }
                return content;
            },

            addContentBlock(type = "json") {
                this.contentBlocks.push({
                    id: crypto.randomUUID(),
                    type,
                    name: "",
                    value: type === "json" ? "{}" : "",
                });
            },

            removeContentBlock(id) {
                this.contentBlocks = this.contentBlocks.filter((block) => block.id !== id);
            },

            parseMetadata() {
                const metadata = JSON.parse(this.metadataText || "{}");
                if (!metadata || Array.isArray(metadata) || typeof metadata !== "object") {
                    throw new Error("Metadata must be a JSON object.");
                }
                return metadata;
            },

            previewPayload() {
                try {
                    return this.rawMode ? JSON.parse(this.rawPayload || "{}") : this.structuredPayload();
                } catch (error) {
                    return { error: error.message };
                }
            },

            toggleRawMode() {
                if (!this.rawMode) this.rawPayload = JSON.stringify(this.structuredPayload(), null, 2);
                this.rawMode = !this.rawMode;
            },

            formatPayload() {
                try {
                    this.rawPayload = JSON.stringify(JSON.parse(this.rawPayload), null, 2);
                    this.error = "";
                } catch (error) {
                    this.error = `Invalid JSON: ${error.message}`;
                }
            },

            async copyPayload() {
                await navigator.clipboard.writeText(JSON.stringify(this.previewPayload(), null, 2));
                this.notice = "Payload copied";
            },

            async sendMessage() {
                this.error = "";
                this.notice = "";
                try {
                    const payload = this.rawMode ? JSON.parse(this.rawPayload) : this.structuredPayload();
                    await this.dispatchMessage(payload);
                } catch (error) {
                    this.error = error.message;
                }
            },

            async sendChatMessage() {
                this.error = "";
                this.notice = "";
                const text = this.chatDraft.trim();
                if (!text) return;
                try {
                    const chatId = this.currentChatId();
                    if (!chatId) throw new Error("Wait for the conversation to be created before replying.");
                    await this.dispatchMessage({
                        chat_id: chatId,
                        subject: null,
                        content: [{ type: "text", text }],
                        metadata: {},
                    });
                    this.chatDraft = "";
                } catch (error) {
                    this.error = error.message;
                }
            },

            async dispatchMessage(input) {
                let payload = input;
                if (!payload.content?.length) throw new Error("At least one content block is required.");
                const traceId = payload.trace_id || this.draftTraceId;
                this.activeChatId = payload.chat_id || null;
                this.pendingTraceId = traceId;
                this.selectedTraceId = traceId;
                this.tab = "chat";

                try {
                    if (this.senderMode === "agent") {
                        if (!this.agentSocket || this.agentSocket.readyState !== WebSocket.OPEN || this.agentStatus !== "connected") {
                            throw new Error("Connect the selected agent before sending.");
                        }
                        if (!payload.chat_id) throw new Error("Agent messages require an existing chat.");
                        this.agentSocket.send(
                            JSON.stringify({
                                id: traceId,
                                method: "message",
                                params: {
                                    chat_id: payload.chat_id,
                                    reply_to_id: payload.reply_to_id || null,
                                    content: payload.content,
                                    metadata: payload.metadata || {},
                                },
                            }),
                        );
                    } else {
                        payload = {
                            ...payload,
                            from: payload.from || {
                                id: this.userExternalId,
                                name: this.userName,
                            },
                        };
                        const response = await fetch(`/tenants/${encodeURIComponent(config.tenant_id)}/messages`, {
                            method: "POST",
                            headers: {
                                "Content-Type": "application/json",
                                "X-Request-ID": traceId,
                            },
                            body: JSON.stringify(payload),
                        });
                        if (!response.ok) throw new Error(`${response.status} ${await response.text()}`);
                    }
                } catch (error) {
                    if (this.pendingTraceId === traceId) this.pendingTraceId = null;
                    throw error;
                }

                this.draftTraceId = crypto.randomUUID();
                this.notice = `Sent · ${traceId.slice(0, 8)}`;
            },

            filteredTraces() {
                const query = this.traceFilter.trim().toLowerCase();
                const key = this.keyFilter.trim().toLowerCase();
                const after = this.traceAfter ? Date.parse(this.traceAfter) : null;
                const before = this.traceBefore ? Date.parse(this.traceBefore) : null;
                return this.traceItems.filter((trace) => {
                    if (key && !trace.keys.some((value) => value.toLowerCase().includes(key))) return false;
                    if (this.traceAgentFilter && !trace.agentIds.includes(this.traceAgentFilter)) return false;
                    if (after !== null && Date.parse(trace.updatedAt) < after) return false;
                    if (before !== null && Date.parse(trace.startedAt) > before) return false;
                    if (!query) return true;
                    return (
                        trace.id.toLowerCase().includes(query) ||
                        JSON.stringify(trace.events).toLowerCase().includes(query)
                    );
                });
            },

            chatConversations() {
                return Reducer.conversations(this.traceItems);
            },

            currentChatTrace() {
                return this.traceItems.find((trace) => trace.id === this.selectedTraceId) || null;
            },

            currentChatId() {
                return this.activeChatId || this.traceChatId(this.currentChatTrace());
            },

            eventChatId(event) {
                return Reducer.eventChatId(event);
            },

            traceChatId(trace) {
                return Reducer.traceChatId(trace);
            },

            conversationEvents() {
                const chatId = this.currentChatId();
                return Reducer.conversationEvents(this.traceItems, chatId, this.selectedTraceId);
            },

            chatMessages() {
                const inboundFingerprints = new Set();
                const messages = [];
                for (const event of this.conversationEvents()) {
                    const message =
                        event.key === "message.inbound"
                            ? event.data?.message
                            : event.key === "message.create"
                              ? event.data?.message
                              : null;
                    if (!message) continue;
                    const actor = message.sent_by || message.created_by || null;
                    const fingerprint = `${actor?.id || "unknown"}:${JSON.stringify(message.content || [])}`;
                    if (event.key === "message.inbound") {
                        inboundFingerprints.add(fingerprint);
                        messages.push({ event, actor, content: message.content || [] });
                    } else if (!inboundFingerprints.has(fingerprint)) {
                        messages.push({ event, actor, content: message.content || [] });
                    }
                }
                return messages;
            },

            chatActivity() {
                return this.conversationEvents().filter(
                    (event) => event.key !== "message.inbound" && event.key !== "message.create",
                );
            },

            selectChatTrace(traceId) {
                const trace = this.traceItems.find((item) => item.id === traceId);
                if (trace) this.selectTrace(trace);
            },

            contentText(block) {
                if (block.type === "text") return block.text;
                if (block.type === "json") return JSON.stringify(block.json, null, 2);
                return block.name || block.uri || block.base64 || "file";
            },

            selectTrace(trace) {
                this.selectedTraceId = trace.id;
                this.activeChatId = this.traceChatId(trace);
                this.selectedEvent = trace.events[trace.events.length - 1] || null;
                this.selectedEntity = null;
                this.applyTraceHighlight();
            },

            currentTraceEvents() {
                return this.traceItems.find((trace) => trace.id === this.selectedTraceId)?.events || [];
            },

            traceLabel(trace) {
                const inbound = trace.events.find((event) => event.key === "message.inbound");
                const text = inbound?.data?.message?.content?.find((item) => item.type === "text")?.text;
                return text || trace.keys[0] || "trace";
            },

            eventDelta(event, index) {
                if (index === 0) return "0 ms";
                const previous = this.currentTraceEvents()[index - 1];
                const delta = Date.parse(event.created_at) - Date.parse(previous.created_at);
                return `${Math.max(0, delta)} ms`;
            },

            inspectEvent(event) {
                this.selectedEvent = event;
                this.selectedEntity = null;
                this.expandedJsonPaths = ["$", "data"];
            },

            inspectorValue() {
                return this.selectedEvent || this.selectedEntity || null;
            },

            inspectorJson() {
                const value = this.inspectorValue();
                if (!value) return "";
                const json = JSON.stringify(value, null, 2);
                const search = this.inspectorSearch.trim().toLowerCase();
                return search && !json.toLowerCase().includes(search) ? "No matching JSON value." : json;
            },

            inspectorTree() {
                const rows = [];
                const walk = (value, path, depth) => {
                    const rowPath = path || "$";
                    const expandable = value !== null && typeof value === "object";
                    const entries = expandable
                        ? Array.isArray(value)
                            ? value.map((item, index) => [index, item])
                            : Object.entries(value)
                        : [];
                    const expanded = this.expandedJsonPaths.includes(rowPath);
                    rows.push({
                        path: rowPath,
                        key: path ? path.split(".").at(-1) : "$",
                        value: expandable ? this.objectPreview(value) : this.primitivePreview(value),
                        valueType: expandable ? "object" : value === null ? "null" : typeof value,
                        depth,
                        expandable,
                        expanded,
                        raw: value,
                    });
                    if (expandable && (expanded || this.inspectorSearch.trim())) {
                        for (const [key, child] of entries) {
                            const childPath = path
                                ? Array.isArray(value)
                                    ? `${path}[${key}]`
                                    : `${path}.${key}`
                                : String(key);
                            walk(child, childPath, depth + 1);
                        }
                    }
                };
                const value = this.inspectorValue();
                if (value) walk(value, "", 0);
                const search = this.inspectorSearch.trim().toLowerCase();
                return search
                    ? rows.filter((row) => `${row.path} ${row.value}`.toLowerCase().includes(search))
                    : rows;
            },

            toggleJsonPath(path) {
                this.expandedJsonPaths = this.expandedJsonPaths.includes(path)
                    ? this.expandedJsonPaths.filter((value) => value !== path)
                    : [...this.expandedJsonPaths, path];
            },

            objectPreview(value) {
                if (Array.isArray(value)) return `Array(${value.length})`;
                const keys = Object.keys(value);
                if (!keys.length) return "{}";
                const preview = keys
                    .slice(0, 2)
                    .map((key) => `${key}: ${this.primitivePreview(value[key], true)}`)
                    .join(", ");
                return `{${preview}${keys.length > 2 ? ", …" : ""}}`;
            },

            primitivePreview(value, compact = false) {
                if (value === null) return "null";
                if (typeof value === "string") {
                    const text = compact && value.length > 26 ? `${value.slice(0, 26)}…` : value;
                    return JSON.stringify(text);
                }
                if (typeof value === "undefined") return "undefined";
                if (typeof value === "object") return Array.isArray(value) ? `Array(${value.length})` : "{…}";
                return String(value);
            },

            async copyInspector() {
                const value = this.inspectorValue();
                if (!value) return;
                await navigator.clipboard.writeText(JSON.stringify(value, null, 2));
                this.notice = "Inspector JSON copied";
            },

            async copyTraceId() {
                if (!this.selectedTraceId) return;
                await navigator.clipboard.writeText(this.selectedTraceId);
                this.notice = "Trace ID copied";
            },

            compactId(value) {
                return value ? `${value.slice(0, 8)}…${value.slice(-4)}` : "—";
            },

            formatTime(value) {
                if (!value) return "—";
                return new Intl.DateTimeFormat(undefined, {
                    hour: "2-digit",
                    minute: "2-digit",
                    second: "2-digit",
                    fractionalSecondDigits: 3,
                }).format(new Date(value));
            },

            formatJson(value) {
                return JSON.stringify(value, null, 2);
            },

            async resetLocalState() {
                this.eventSocket?.close();
                this.agentSocket?.close();
                this.database?.close();
                await new Promise((resolve, reject) => {
                    const request = indexedDB.deleteDatabase(
                        `mage-console-${config.reducer_version}-${config.tenant_id}`,
                    );
                    request.onsuccess = resolve;
                    request.onerror = () => reject(request.error);
                    request.onblocked = resolve;
                });
                window.location.reload();
            },
        };
    }

    document.addEventListener("alpine:init", () => {
        window.Alpine.data("mageConsole", createConsole);
    });
})();
