import { Component, OnInit } from '@angular/core';

import { ForceGraphComponent } from '../force-graph/force-graph.component';
import { TauriService, Bike, ForceNode } from '../../services/tauri.service';

/**
 * Deliverer Graph Page
 *
 * # Purpose
 * Full-page view showing the force graph for a selected deliverer.
 * Allows users to:
 * - Select a deliverer from the fleet
 * - View their deliveries and issues as a force-directed graph
 * - Click nodes for details
 * - Drag nodes to rearrange
 *
 * # Data Flow
 * 1. On init, fetch fleet data (list of bikes/deliverers)
 * 2. User selects a deliverer
 * 3. ForceGraphComponent loads and displays the graph
 * 4. User interactions are handled by ForceGraphComponent
 */
@Component({
  selector: 'app-deliverer-graph-page',
  standalone: true,
  imports: [ForceGraphComponent],
  templateUrl: './deliverer-graph-page.component.html',
  styleUrls: ['./deliverer-graph-page.component.scss'],
})
export class DelivererGraphPageComponent implements OnInit {
  // Fleet data
  bikes: Bike[] = [];
  selectedBikeId: string | null = null;

  // Loading state
  loading = true;
  error: string | null = null;

  // Selected node details (for info panel)
  selectedNode: ForceNode | null = null;

  constructor(private tauriService: TauriService) {}

  async ngOnInit(): Promise<void> {
    await this.loadFleetData();
  }

  /**
   * Load fleet data to populate deliverer selector
   */
  async loadFleetData(): Promise<void> {
    this.loading = true;
    this.error = null;

    try {
      // First ensure database is initialized
      const isInitialized = await this.tauriService.isDatabaseInitialized();
      if (!isInitialized) {
        await this.tauriService.initDatabase();
      }

      // Fetch fleet data
      this.bikes = await this.tauriService.getFleetData();

      // Auto-select first bike if available
      if (this.bikes.length > 0 && !this.selectedBikeId) {
        this.selectedBikeId = this.bikes[0].id;
      }
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Failed to load fleet data';
      console.error('Fleet data error:', err);
    } finally {
      this.loading = false;
    }
  }

  /**
   * Handle deliverer selection change
   */
  onBikeSelect(event: Event): void {
    const select = event.target as HTMLSelectElement;
    this.selectedBikeId = select.value || null;
    this.selectedNode = null;
  }

  /**
   * Handle node click from force graph
   */
  onNodeClick(node: ForceNode): void {
    this.selectedNode = node;
  }

  /**
   * Handle node double-click (e.g., drill down)
   */
  onNodeDblClick(node: ForceNode): void {
    // Could navigate to delivery/issue detail page
    console.log('Double-clicked node:', node);
  }

  /**
   * Get display name for a bike
   */
  getBikeName(bike: Bike): string {
    return `${bike.name} (${bike.status})`;
  }

  /**
   * Get details for selected node (for info panel)
   */
  getNodeDetails(): { label: string; value: string }[] {
    if (!this.selectedNode) return [];

    const details: { label: string; value: string }[] = [
      { label: 'Type', value: this.selectedNode.nodeType },
      { label: 'ID', value: this.selectedNode.id },
    ];

    switch (this.selectedNode.data.type) {
      case 'deliverer':
        details.push(
          { label: 'Name', value: this.selectedNode.data.name || '' },
          { label: 'Status', value: this.selectedNode.data.status || '' }
        );
        break;
      case 'delivery':
        details.push(
          { label: 'Customer', value: this.selectedNode.data.customer || '' },
          { label: 'Status', value: this.selectedNode.data.status || '' },
          { label: 'Rating', value: this.selectedNode.data.rating?.toString() || 'N/A' }
        );
        break;
      case 'issue':
        details.push(
          { label: 'Category', value: this.selectedNode.data.category || '' },
          { label: 'Reporter', value: this.selectedNode.data.reporter || '' },
          { label: 'Resolved', value: this.selectedNode.data.resolved ? 'Yes' : 'No' }
        );
        break;
    }

    return details;
  }

  /**
   * Clear selected node
   */
  clearSelection(): void {
    this.selectedNode = null;
  }
}
