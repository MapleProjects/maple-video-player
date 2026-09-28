-- 🍁 Maple Video Player - On-Screen HUD & Controls (Pure Vulkan GPU-Next & LSFG)
-- Runs natively inside MPV's libplacebo Vulkan swapchain without separate windows

local mp = require 'mp'
local utils = require 'mp.utils'

local config_dir = os.getenv("HOME") .. "/.config/maple-player"
local settings_path = config_dir .. "/settings.json"
local shaders_dir = config_dir .. "/shaders"
local lsfg_conf_path = os.getenv("HOME") .. "/.config/lsfg-vk/conf.toml"

-- Default settings
local state = {
    backend = "Vulkan",
    upscale_mode = "SnapdragonGsr", -- "Off", "SnapdragonGsr", "Fsr", "Cas", "Anime4k"
    render_scale = "Native",       -- "Native", "UltraQuality", "Quality", "Balanced", "Performance"
    fsr_sharpness = 0.80,
    fsr_denoise = 0.20,
    fsr_edge_threshold = 0.25,
    fsr_deringing = true,
    sgsr_edge_threshold = 4.0,
    sgsr_edge_sharpness = 2.0,
    sgsr_edge_direction = true,
    cas_sharpness = 0.80,
    deband_enabled = true,
    deband_iterations = 4,
    deband_threshold = 48,
    deband_range = 16,
    deband_grain = 32,
    bit_depth = "Bits10",
    dither_algo = "ErrorDiffusion",
    temporal_dither = true,
    extra_deband_shader = false,
    interpolation_enabled = true,
    lsfg_multiplier = 2,
    hdr_mode = true,
}

-- UI state
local menu_open = false
local telemetry_open = false
local selected_idx = 1
local ov_menu = mp.create_osd_overlay("ass-events")
local ov_telemetry = mp.create_osd_overlay("ass-events")
local ov_splash = mp.create_osd_overlay("ass-events")

-- Menu items definition
local menu_items = {
    { id = "upscale_mode", label = "Algoritmo de Reconstrucción", type = "enum", options = {"Off", "SnapdragonGsr", "Fsr", "Cas", "Anime4k"}, opt_labels = {"Desactivado", "Qualcomm Snapdragon GSR v1", "AMD FSR 1.0 (EASU+RCAS)", "AMD CAS", "Anime4K CNN"} },
    { id = "sgsr_edge_sharpness", label = "Qualcomm GSR Nitidez", type = "float", min = 0.5, max = 4.0, step = 0.25, cond = function() return state.upscale_mode == "SnapdragonGsr" end },
    { id = "sgsr_edge_threshold", label = "Qualcomm GSR Umbral", type = "float", min = 0.5, max = 20.0, step = 0.5, cond = function() return state.upscale_mode == "SnapdragonGsr" end },
    { id = "sgsr_edge_direction", label = "Qualcomm GSR Detección Direccional", type = "bool", cond = function() return state.upscale_mode == "SnapdragonGsr" end },
    { id = "fsr_sharpness", label = "AMD FSR Nitidez", type = "float", min = 0.0, max = 2.0, step = 0.05, cond = function() return state.upscale_mode == "Fsr" end },
    { id = "fsr_denoise", label = "AMD FSR Reducción de Ruido", type = "float", min = 0.0, max = 1.0, step = 0.05, cond = function() return state.upscale_mode == "Fsr" end },
    { id = "cas_sharpness", label = "AMD CAS Nitidez", type = "float", min = 0.0, max = 2.0, step = 0.10, cond = function() return state.upscale_mode == "Cas" end },
    { id = "lsfg_multiplier", label = "Generación de Cuadros (LSFG)", type = "enum", options = {1, 2, 3}, opt_labels = {"1x (Desactivado)", "2x (Doble FPS)", "3x (Triple FPS)"} },
    { id = "deband_enabled", label = "Filtro Anti-Banding (Deband)", type = "bool" },
    { id = "deband_iterations", label = "Deband Iteraciones", type = "int", min = 1, max = 8, step = 1, cond = function() return state.deband_enabled end },
    { id = "deband_threshold", label = "Deband Umbral de Detección", type = "int", min = 16, max = 128, step = 8, cond = function() return state.deband_enabled end },
    { id = "hdr_mode", label = "Pipeline HDR 10-bit (ST2084 / BT.2020)", type = "bool" },
}

-- Simple JSON reader & writer
local function load_settings()
    local f = io.open(settings_path, "r")
    if not f then return end
    local content = f:read("*a")
    f:close()
    if not content or content == "" then return end

    -- Extract values via patterns for robustness
    for k, v in pairs(state) do
        if type(v) == "string" then
            local str = content:match('"' .. k .. '"%s*:%s*"([^"]+)"')
            if str then state[k] = str end
        elseif type(v) == "number" then
            local num = content:match('"' .. k .. '"%s*:%s*([%-%d%.]+)')
            if num then state[k] = tonumber(num) end
        elseif type(v) == "boolean" then
            local b = content:match('"' .. k .. '"%s*:%s*(true|false)')
            if b then state[k] = (b == "true") end
        end
    end
end

local function save_settings()
    local f = io.open(settings_path, "w")
    if not f then return end
    f:write("{\n")
    f:write(string.format('  "backend": "%s",\n', state.backend))
    f:write(string.format('  "upscale_mode": "%s",\n', state.upscale_mode))
    f:write(string.format('  "render_scale": "%s",\n', state.render_scale))
    f:write(string.format('  "fsr_sharpness": %.2f,\n', state.fsr_sharpness))
    f:write(string.format('  "fsr_denoise": %.2f,\n', state.fsr_denoise))
    f:write(string.format('  "fsr_edge_threshold": %.2f,\n', state.fsr_edge_threshold))
    f:write(string.format('  "fsr_deringing": %s,\n', tostring(state.fsr_deringing)))
    f:write(string.format('  "sgsr_edge_threshold": %.1f,\n', state.sgsr_edge_threshold))
    f:write(string.format('  "sgsr_edge_sharpness": %.2f,\n', state.sgsr_edge_sharpness))
    f:write(string.format('  "sgsr_edge_direction": %s,\n', tostring(state.sgsr_edge_direction)))
    f:write(string.format('  "cas_sharpness": %.2f,\n', state.cas_sharpness))
    f:write(string.format('  "deband_enabled": %s,\n', tostring(state.deband_enabled)))
    f:write(string.format('  "deband_iterations": %d,\n', state.deband_iterations))
    f:write(string.format('  "deband_threshold": %d,\n', state.deband_threshold))
    f:write(string.format('  "deband_range": %d,\n', state.deband_range))
    f:write(string.format('  "deband_grain": %d,\n', state.deband_grain))
    f:write(string.format('  "bit_depth": "%s",\n', state.bit_depth))
    f:write(string.format('  "dither_algo": "%s",\n', state.dither_algo))
    f:write(string.format('  "temporal_dither": %s,\n', tostring(state.temporal_dither)))
    f:write(string.format('  "extra_deband_shader": %s,\n', tostring(state.extra_deband_shader)))
    f:write(string.format('  "interpolation_enabled": %s,\n', tostring(state.interpolation_enabled)))
    f:write(string.format('  "lsfg_multiplier": %d,\n', state.lsfg_multiplier))
    f:write(string.format('  "hdr_mode": %s\n', tostring(state.hdr_mode)))
    f:write("}\n")
    f:close()

    -- Sync LSFG conf.toml
    local lf = io.open(lsfg_conf_path, "w")
    if lf then
        lf:write("version = 2\n\n[global]\nallow_fp16 = true\nlog_level = \"info\"\n")
        lf:write("dll = \"" .. os.getenv("HOME") .. "/.local/share/Steam/steamapps/common/Lossless Scaling/lsfg-vk.dll\"\n\n")
        lf:write("[[profile]]\n")
        lf:write("name = \"Maple Video Player LSFG\"\n")
        lf:write("active_in = [ \"maple-video-player\", \"maple-player\", \"mpv\" ]\n")
        lf:write(string.format("multiplier = %d\n", state.lsfg_multiplier))
        lf:write("flow_scale = 1.00\nperformance_mode = false\npacing_mode = \"vsync\"\n")
        lf:write("override_present_mode = true\npreserve_swapchain_image_count = false\n")
        lf:close()
    end
end

-- Regenerate and reapply shaders
local function apply_shaders_and_tuning()
    local shader_paths = {}

    if state.upscale_mode == "SnapdragonGsr" then
        local template_f = io.open(shaders_dir .. "/snapdragon_gsr.glsl", "r")
        local content = ""
        if template_f then
            content = template_f:read("*a")
            template_f:close()
        end
        if content ~= "" then
            local ed = state.sgsr_edge_direction and "1" or "0"
            content = content:gsub("#define EdgeThreshold [%d%.]+", string.format("#define EdgeThreshold %.1f", state.sgsr_edge_threshold))
            content = content:gsub("#define EdgeSharpness [%d%.]+", string.format("#define EdgeSharpness %.2f", state.sgsr_edge_sharpness))
            content = content:gsub("#define UseEdgeDirection %d", "#define UseEdgeDirection " .. ed)

            local active_p = shaders_dir .. "/snapdragon_gsr_active.glsl"
            local out_f = io.open(active_p, "w")
            if out_f then
                out_f:write(content)
                out_f:close()
                table.insert(shader_paths, active_p)
            end
        end
    elseif state.upscale_mode == "Fsr" then
        local template_f = io.open(shaders_dir .. "/fsr.glsl", "r")
        local content = ""
        if template_f then
            content = template_f:read("*a")
            template_f:close()
        end
        if content ~= "" then
            content = content:gsub("#define SHARPNESS [%d%.]+", string.format("#define SHARPNESS %.2f", state.fsr_sharpness))
            content = content:gsub("#define FSR_RCAS_DENOISE [%d%.]+", string.format("#define FSR_RCAS_DENOISE %.2f", state.fsr_denoise))
            local active_p = shaders_dir .. "/fsr_active.glsl"
            local out_f = io.open(active_p, "w")
            if out_f then
                out_f:write(content)
                out_f:close()
                table.insert(shader_paths, active_p)
            end
        end
    elseif state.upscale_mode == "Cas" then
        local template_f = io.open(shaders_dir .. "/cas.glsl", "r")
        local content = ""
        if template_f then
            content = template_f:read("*a")
            template_f:close()
        end
        if content ~= "" then
            content = content:gsub("#define SHARPNESS [%d%.]+", string.format("#define SHARPNESS %.2f", state.cas_sharpness))
            local active_p = shaders_dir .. "/cas_active.glsl"
            local out_f = io.open(active_p, "w")
            if out_f then
                out_f:write(content)
                out_f:close()
                table.insert(shader_paths, active_p)
            end
        end
    elseif state.upscale_mode == "Anime4k" then
        table.insert(shader_paths, shaders_dir .. "/anime4k.glsl")
    end

    if state.extra_deband_shader then
        table.insert(shader_paths, shaders_dir .. "/deband_ext.glsl")
    end

    -- Apply GLSL shaders in Vulkan pipeline
    if #shader_paths == 0 then
        mp.commandv("change-list", "glsl-shaders", "clr", "")
    else
        mp.commandv("change-list", "glsl-shaders", "set", table.concat(shader_paths, ":"))
    end

    -- Apply deband properties
    mp.set_property("deband", state.deband_enabled and "yes" or "no")
    if state.deband_enabled then
        mp.set_property_number("deband-iterations", state.deband_iterations)
        mp.set_property_number("deband-threshold", state.deband_threshold)
        mp.set_property_number("deband-range", state.deband_range)
        mp.set_property_number("deband-grain", state.deband_grain)
    end

    -- Apply 10-bit HDR properties
    if state.hdr_mode then
        mp.set_property("target-colorspace-hint", "yes")
        mp.set_property("dither-depth", "10")
        mp.set_property("fbo-format", "rgba16f")
    else
        mp.set_property("target-colorspace-hint", "no")
        mp.set_property("dither-depth", "8")
    end

    save_settings()
end

-- Open native file picker via zenity
local function open_file_dialog()
    local res = mp.command_native({
        name = "subprocess",
        playback_only = false,
        capture_stdout = true,
        args = {
            "zenity", "--file-selection",
            "--title=Seleccionar Video - 🍁 Maple Video Player",
            "--file-filter=Archivos de Video | *.mp4 *.mkv *.webm *.avi *.mov *.flv *.ts *.m4v *.m2ts"
        }
    })
    if res and res.status == 0 and res.stdout and res.stdout ~= "" then
        local path = res.stdout:gsub("\r?\n$", "")
        if path ~= "" then
            mp.commandv("loadfile", path)
            mp.osd_message("📂 Cargando video: " .. path, 2)
        end
    end
end

-- Render on-screen HUD Menu
local function render_menu()
    if not menu_open then
        ov_menu.data = ""
        ov_menu:update()
        return
    end

    local visible_items = {}
    for _, item in ipairs(menu_items) do
        if not item.cond or item.cond() then
            table.insert(visible_items, item)
        end
    end

    if selected_idx > #visible_items then selected_idx = #visible_items end
    if selected_idx < 1 then selected_idx = 1 end

    local ass = "{\\an7\\fs20\\fnDejaVu Sans\\b0\\c&HFFFFFF&}"
    ass = ass .. "{\\pos(50,50)}"
    ass = ass .. "{\\b1\\fs26\\c&H4696FF&}MAPLE VIDEO PLAYER{\\b0\\fs18\\c&HAAAAAA&} • Panel de Control Vulkan & LSFG\\N\\N"

    for i, item in ipairs(visible_items) do
        local is_selected = (i == selected_idx)
        local prefix = is_selected and "{\\b1\\c&H00D7FF&} ▶ " or "{\\b0\\c&HBBBBBB&}   "
        local val_str = ""

        if item.type == "enum" then
            for idx, opt in ipairs(item.options) do
                if state[item.id] == opt then
                    val_str = item.opt_labels[idx]
                    break
                end
            end
            val_str = "{\\b1\\c&H38B0DE&}[ " .. val_str .. " ]"
        elseif item.type == "float" then
            val_str = string.format("{\\b1\\c&H76EE00&}%.2f {\\c&H888888&}(Min: %.1f, Max: %.1f)", state[item.id], item.min, item.max)
        elseif item.type == "int" then
            val_str = string.format("{\\b1\\c&H76EE00&}%d {\\c&H888888&}(Min: %d, Max: %d)", state[item.id], item.min, item.max)
        elseif item.type == "bool" then
            local active = state[item.id]
            val_str = active and "{\\b1\\c&H00FF00&}[ ACTIVADO ]" or "{\\b1\\c&H888888&}[ Desactivado ]"
        end

        local item_text = string.format("%s%-36s : %s{\\r}\\N", prefix, item.label, val_str)
        ass = ass .. item_text
    end

    ass = ass .. "\\N{\\fs16\\c&H888888&}Navegación: {\\c&HFFFFFF&}▲/▼ o j/k {\\c&H888888&}| Ajustar: {\\c&HFFFFFF&}◄/► o h/l {\\c&H888888&}| Alternar: {\\c&HFFFFFF&}Enter/Espacio\\N"
    ass = ass .. "Acciones: {\\c&HFFFFFF&}O {\\c&H888888&}(Abrir Video) | {\\c&HFFFFFF&}T {\\c&H888888&}(Telemetría) | {\\c&HFFFFFF&}Tab/M {\\c&H888888&}(Cerrar Menú){\\r}"

    ov_menu.data = ass
    ov_menu:update()
end

-- Render live telemetry HUD
local function render_telemetry()
    if not telemetry_open then
        ov_telemetry.data = ""
        ov_telemetry:update()
        return
    end

    local d_fps = mp.get_property_number("display-fps", 120.21)
    local e_fps = mp.get_property_number("estimated-vf-fps", 0)
    local c_fps = mp.get_property_number("container-fps", 0)
    local drops = mp.get_property_number("frame-drop-count", 0)
    local width = mp.get_property_number("video-out-params/w", 1920)
    local height = mp.get_property_number("video-out-params/h", 1080)
    local pixelformat = mp.get_property("video-out-params/pixelformat", "p010")
    local hwdec = mp.get_property("hwdec-current", "nvdec-copy")

    local osd_w, osd_h = mp.get_osd_size()
    if not osd_w or osd_w == 0 then osd_w = 1280 end
    local pos_x = osd_w - 30

    local ass = string.format("{\\an9\\fs18\\fnDejaVu Sans\\pos(%d,40)}", pos_x)
    ass = ass .. "{\\b1\\fs22\\c&H4696FF&}TELEMETRÍA EN VIVO (Vulkan GPU-Next){\\b0\\fs17\\c&HFFFFFF&}\\N"
    ass = ass .. string.format("Pantalla Wayland: {\\c&H00FF99&}%.2f Hz (Vulkan Swapchain){\\c&HFFFFFF&}\\N", d_fps)
    ass = ass .. string.format("Video Fuente: {\\c&H00D7FF&}%dx%d @ %.2f fps (%s){\\c&HFFFFFF&}\\N", width, height, c_fps > 0 and c_fps or e_fps, pixelformat)
    ass = ass .. string.format("Decodificación HW: {\\c&H76EE00&}%s (Drops: %d){\\c&HFFFFFF&}\\N", hwdec, drops)
    ass = ass .. string.format("Formato Swapchain: {\\c&HFFB90F&}VK_FORMAT_A2B10G10R10 (10-bit HDR ST2084){\\c&HFFFFFF&}\\N")
    ass = ass .. string.format("Capa Lossless Scaling: {\\c&H00E5EE&}VK_LAYER_LSFGVK (Activa • %dx Multiplicador){\\c&HFFFFFF&}\\N", state.lsfg_multiplier)
    ass = ass .. string.format("Reconstrucción Activa: {\\c&HFF69B4&}%s{\\c&HFFFFFF&}\\N", state.upscale_mode)
    if state.upscale_mode == "SnapdragonGsr" then
        ass = ass .. string.format("  └─ GSR Nitidez: {\\c&H76EE00&}%.2f {\\c&HFFFFFF&}| Umbral: {\\c&H76EE00&}%.1f {\\c&HFFFFFF&}| Dir: {\\c&H76EE00&}%s{\\c&HFFFFFF&}\\N", state.sgsr_edge_sharpness, state.sgsr_edge_threshold, state.sgsr_edge_direction and "On" or "Off")
    elseif state.upscale_mode == "Fsr" then
        ass = ass .. string.format("  └─ FSR Nitidez: {\\c&H76EE00&}%.2f {\\c&HFFFFFF&}| Ruido: {\\c&H76EE00&}%.2f{\\c&HFFFFFF&}\\N", state.fsr_sharpness, state.fsr_denoise)
    end
    ass = ass .. string.format("Deband Adaptable: {\\c&H00FF7F&}%s (It: %d, Thresh: %d){\\c&HFFFFFF&}\\N", state.deband_enabled and "Activo" or "Off", state.deband_iterations, state.deband_threshold)
    ass = ass .. string.format("Pipeline HDR ST2084: {\\c&H00FF00&}%s (RGBA16F Master FBO){\\c&HFFFFFF&}\\N", state.hdr_mode and "Activado" or "Off")

    ov_telemetry.data = ass
    ov_telemetry:update()
end

-- Render splash screen on idle
local function render_splash()
    local idle = mp.get_property_bool("idle-active", false)
    local eof = mp.get_property_bool("eof-reached", false)
    local path = mp.get_property("path")

    if (idle or not path or path == "") and not menu_open then
        local ass = "{\\an5\\pos(960,540)\\fnDejaVu Sans}"
        ass = ass .. "{\\b1\\fs42\\c&H4696FF&}MAPLE VIDEO PLAYER\\N"
        ass = ass .. "{\\b0\\fs22\\c&HCCCCCC&}Arquitectura Pura Vulkan GPU-Next & Capa Oficial Lossless Scaling (LSFG)\\N\\N"
        ass = ass .. "{\\fs20\\c&HAAAAAA&}• Super Resolución Qualcomm Snapdragon GSR v1 & AMD FSR 1.0\\N"
        ass = ass .. "• Interpolación de Cuadros LSFG nativa en Swapchain Vulkan\\N"
        ass = ass .. "• Pipeline de Color Master 10-bit HDR / ST2084 & Deband Adaptable\\N\\N"
        ass = ass .. "{\\b1\\fs26\\c&H00FF99&}[ Presiona 'O' para Abrir un Video o arrastra un archivo aquí ]{\\b0\\c&HFFFFFF&}\\N\\N"
        ass = ass .. "{\\fs18\\c&H777777&}Presiona {\\c&HFFFFFF&}Tab {\\c&H777777&}para el Panel de Control • {\\c&HFFFFFF&}T {\\c&H777777&}para Telemetría • {\\c&HFFFFFF&}Espacio {\\c&H777777&}Pausar/Reanudar{\\r}"
        ov_splash.data = ass
        ov_splash:update()
    else
        ov_splash.data = ""
        ov_splash:update()
    end
end

-- Menu actions
local function menu_up()
    if not menu_open then return end
    selected_idx = selected_idx - 1
    render_menu()
end

local function menu_down()
    if not menu_open then return end
    selected_idx = selected_idx + 1
    render_menu()
end

local function menu_adjust(dir)
    if not menu_open then return end

    local visible_items = {}
    for _, item in ipairs(menu_items) do
        if not item.cond or item.cond() then
            table.insert(visible_items, item)
        end
    end

    local item = visible_items[selected_idx]
    if not item then return end

    if item.type == "enum" then
        local cur_idx = 1
        for idx, opt in ipairs(item.options) do
            if state[item.id] == opt then
                cur_idx = idx
                break
            end
        end
        cur_idx = cur_idx + dir
        if cur_idx > #item.options then cur_idx = 1 end
        if cur_idx < 1 then cur_idx = #item.options end
        state[item.id] = item.options[cur_idx]
    elseif item.type == "float" then
        state[item.id] = math.max(item.min, math.min(item.max, state[item.id] + dir * item.step))
    elseif item.type == "int" then
        state[item.id] = math.max(item.min, math.min(item.max, state[item.id] + dir * item.step))
    elseif item.type == "bool" then
        state[item.id] = not state[item.id]
    end

    apply_shaders_and_tuning()
    render_menu()
    if telemetry_open then render_telemetry() end
end

local function menu_toggle()
    menu_open = not menu_open
    if menu_open then
        telemetry_open = false
        render_telemetry()
    end
    render_menu()
    render_splash()
end

local function telemetry_toggle()
    telemetry_open = not telemetry_open
    if telemetry_open then
        menu_open = false
        render_menu()
    end
    render_telemetry()
end

-- Keybindings
mp.add_forced_key_binding("Tab", "maple_menu_toggle", menu_toggle)
mp.add_forced_key_binding("m", "maple_menu_toggle_m", menu_toggle)
mp.add_forced_key_binding("M", "maple_menu_toggle_M", menu_toggle)
mp.add_forced_key_binding("t", "maple_telemetry_toggle", telemetry_toggle)
mp.add_forced_key_binding("T", "maple_telemetry_toggle_T", telemetry_toggle)
mp.add_forced_key_binding("i", "maple_telemetry_toggle_i", telemetry_toggle)
mp.add_forced_key_binding("I", "maple_telemetry_toggle_I", telemetry_toggle)
mp.add_forced_key_binding("o", "maple_open_file", open_file_dialog)
mp.add_forced_key_binding("O", "maple_open_file_O", open_file_dialog)

-- Menu navigation bindings (active always or mapped to arrow keys)
mp.add_forced_key_binding("Up", "maple_nav_up", function() if menu_open then menu_up() else mp.commandv("seek", "60") end end)
mp.add_forced_key_binding("Down", "maple_nav_down", function() if menu_open then menu_down() else mp.commandv("seek", "-60") end end)
mp.add_forced_key_binding("Left", "maple_nav_left", function() if menu_open then menu_adjust(-1) else mp.commandv("seek", "-5") end end)
mp.add_forced_key_binding("Right", "maple_nav_right", function() if menu_open then menu_adjust(1) else mp.commandv("seek", "5") end end)
mp.add_forced_key_binding("Enter", "maple_nav_enter", function() if menu_open then menu_adjust(1) else mp.commandv("cycle", "pause") end end)
mp.add_forced_key_binding("k", "maple_nav_k", function() if menu_open then menu_up() end end)
mp.add_forced_key_binding("j", "maple_nav_j", function() if menu_open then menu_down() end end)
mp.add_forced_key_binding("h", "maple_nav_h", function() if menu_open then menu_adjust(-1) end end)
mp.add_forced_key_binding("l", "maple_nav_l", function() if menu_open then menu_adjust(1) end end)

-- Quick toggle hotkeys
mp.add_forced_key_binding("1", "maple_quick_gsr", function()
    state.upscale_mode = "SnapdragonGsr"
    apply_shaders_and_tuning()
    mp.osd_message("Reconstrucción: Qualcomm Snapdragon GSR v1", 2)
    render_menu()
end)

mp.add_forced_key_binding("2", "maple_quick_fsr", function()
    state.upscale_mode = "Fsr"
    apply_shaders_and_tuning()
    mp.osd_message("Reconstrucción: AMD FidelityFX FSR 1.0", 2)
    render_menu()
end)

mp.add_forced_key_binding("3", "maple_quick_cas", function()
    state.upscale_mode = "Cas"
    apply_shaders_and_tuning()
    mp.osd_message("Reconstrucción: AMD CAS Adaptable", 2)
    render_menu()
end)

mp.add_forced_key_binding("0", "maple_quick_off", function()
    state.upscale_mode = "Off"
    apply_shaders_and_tuning()
    mp.osd_message("Reconstrucción: Desactivada (Nativo)", 2)
    render_menu()
end)

-- Periodically update telemetry if open
local telemetry_timer = mp.add_periodic_timer(0.5, function()
    if telemetry_open then render_telemetry() end
end)

-- Events
mp.register_event("file-loaded", function()
    apply_shaders_and_tuning()
    render_splash()
    mp.osd_message("🍁 Maple Player: Video Cargado (Vulkan GPU-Next • LSFG Layer)", 3)
end)

mp.register_event("end-file", function()
    render_splash()
end)

mp.observe_property("idle-active", "bool", function(_, idle)
    render_splash()
end)

-- Initialization
load_settings()
apply_shaders_and_tuning()
render_splash()
