// ═══════════════════════════════════════════════
//  Plugin: Live Touchpad
//  Shows live touchpad contacts
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'live-touchpad',
  name: 'Live Touchpad',
  defaultWidth: 'half',
  defaultOrder: 81,

  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-title" style="display: flex; justify-content: space-between; align-items: center;">
        <div><span class="dot" style="background:var(--cyan)"></span> Live Touchpad Feed</div>
        <button id="calibrate-touchpad-btn" class="btn" style="padding: 4px 10px; font-size: 0.75rem;">Calibrate Bounds</button>
      </div>
      <div id="live-touchpad-content" style="position: relative; width: 100%; aspect-ratio: 1.5; background: #10121a; border-radius: 12px; overflow: hidden; border: 1px solid rgba(255,255,255,0.04);">
        <canvas id="live-touchpad-canvas" width="900" height="600" style="width: 100%; height: 100%;"></canvas>
      </div>
    </div>`;
    
    this.canvas = document.getElementById('live-touchpad-canvas');
    this.ctx = this.canvas.getContext('2d');
    this.max_x = 3000;
    this.max_y = 2000;

    document.getElementById('calibrate-touchpad-btn').onclick = () => {
      this.max_x = 100;
      this.max_y = 100;
      alert("Bounds reset! Now swirl your fingers along all edges of the touchpad to calibrate the max size.");
    };

    this.polling = null;
    this.drawEmpty();
    this.startLiveFeed();
  },

  drawEmpty() {
    if (!this.ctx) return;
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    this.ctx.fillStyle = '#6b7094';
    this.ctx.font = '14px system-ui';
    this.ctx.textAlign = 'center';
    this.ctx.fillText("Waiting for touchpad input...", this.canvas.width / 2, this.canvas.height / 2);
  },

  startLiveFeed() {
    this.connectWebSocket();
  },

  connectWebSocket() {
    const token = typeof _statsToken !== 'undefined' ? _statsToken : null;
    if(!token) {
        setTimeout(() => this.connectWebSocket(), 500);
        return;
    }

    try {
      this.ws = new WebSocket(`ws://127.0.0.1:9899/?token=${token}`);
    } catch(e) {
      setTimeout(() => this.connectWebSocket(), 2000);
      return;
    }
    
    this.ws.onmessage = (event) => {
        try {
            if (!event.data || event.data.trim() === '[]' || event.data.trim() === '') {
                this.drawEmpty();
                return;
            }
            
            const contacts = JSON.parse(event.data);
            if (!contacts || contacts.length === 0) {
                this.drawEmpty();
                return;
            }

            this.drawContacts(contacts);
        } catch(e) {}
    };

    this.ws.onerror = () => {};

    this.ws.onclose = () => {
        setTimeout(() => this.connectWebSocket(), 2000); 
    };
  },

  drawContacts(contacts) {
    if (!this.ctx) return;
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    if (!contacts || contacts.length === 0) {
        this.drawEmpty();
        return;
    }

    this.ctx.globalCompositeOperation = 'lighter';

    for (let i = 0; i < contacts.length; i++) {
        const touch = contacts[i];
        
        if (touch.x > this.max_x) this.max_x = touch.x;
        if (touch.y > this.max_y) this.max_y = touch.y;
        
        const x = (touch.x / this.max_x) * this.canvas.width;
        const y = (touch.y / this.max_y) * this.canvas.height;
        
        this.ctx.beginPath();
        const hueBase = touch.id * 80;
        this.ctx.fillStyle = `hsla(${200 + hueBase}, 100%, 65%, 0.8)`;
        this.ctx.shadowColor = `hsl(${200 + hueBase}, 100%, 60%)`;
        this.ctx.shadowBlur = 15;
        this.ctx.arc(x, y, 22, 0, 2 * Math.PI);
        this.ctx.fill();
        this.ctx.shadowBlur = 0;
        
        this.ctx.fillStyle = '#10121a';
        this.ctx.font = '12px system-ui';
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        this.ctx.fillText(touch.id, x, y);
    }
    
    this.ctx.globalCompositeOperation = 'source-over';
  },

  update(d) {
    // Updates are handled by startLiveFeed directly
  }
});
