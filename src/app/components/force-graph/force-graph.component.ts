import {
  Component,
  ElementRef,
  EventEmitter,
  Input,
  OnChanges,
  OnDestroy,
  OnInit,
  Output,
  SimpleChanges,
  ViewChild,
} from '@angular/core';

import {
  ForceNode,
  ForceLink,
  TauriService,
} from '../../services/tauri.service';

/**
 * Force Graph Component
 *
 * # Purpose
 * Renders a force-directed graph visualization showing the relationships
 * between a deliverer (courier), their deliveries, and associated issues.
 *
 * # Architecture
 * - Layout computation happens in Rust/Tauri backend (Fjädra)
 * - This component only RENDERS pre-computed positions
 * - Drag events are sent to backend for re-computation
 *
 * # Why SVG?
 * - Vector graphics scale perfectly at any zoom level
 * - Native DOM events for interaction (drag, click, hover)
 * - CSS styling for visual appearance
 * - Accessibility: screen readers can parse SVG text
 *
 * # Node Types
 * - Deliverer (center): Large node representing the courier/bike
 * - Delivery: Medium nodes connected to deliverer
 * - Issue: Small nodes connected to delivery or directly to deliverer
 */
@Component({
  selector: 'app-force-graph',
  standalone: true,
  imports: [],
  templateUrl: './force-graph.component.html',
  styleUrls: ['./force-graph.component.scss'],
})
export class ForceGraphComponent implements OnInit, OnChanges, OnDestroy {
  /**
   * ID of the bike/deliverer to visualize
   * Changing this will fetch new graph data from backend
   */
  @Input() bikeId: string | null = null;

  /**
   * Width of the SVG viewport (pixels)
   */
  @Input() width = 800;

  /**
   * Height of the SVG viewport (pixels)
   */
  @Input() height = 600;

  /**
   * Emitted when a node is clicked
   */
  @Output() nodeClick = new EventEmitter<ForceNode>();

  /**
   * Emitted when a node is double-clicked
   */
  @Output() nodeDblClick = new EventEmitter<ForceNode>();

  @ViewChild('svgElement') svgElement!: ElementRef<SVGSVGElement>;

  // Graph data from backend
  nodes: ForceNode[] = [];
  links: ForceLink[] = [];
  viewBox = '0 0 800 600';

  // Loading and error states
  loading = false;
  error: string | null = null;

  // Drag state
  private draggedNode: ForceNode | null = null;
  private dragStartX = 0;
  private dragStartY = 0;

  constructor(private tauriService: TauriService) {}

  ngOnInit(): void {
    if (this.bikeId) {
      this.loadGraphData();
    }
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['bikeId'] && !changes['bikeId'].firstChange) {
      this.loadGraphData();
    }
    if (changes['width'] || changes['height']) {
      this.updateViewBox();
    }
  }

  ngOnDestroy(): void {
    // Clean up any pending operations
    this.draggedNode = null;
  }

  /**
   * Load graph data from Tauri backend
   *
   * Flow:
   * 1. Call get_force_graph_layout command
   * 2. Backend fetches deliverer, deliveries, issues
   * 3. Backend runs Fjädra simulation
   * 4. Backend returns computed positions
   * 5. Component renders nodes and links
   */
  async loadGraphData(): Promise<void> {
    if (!this.bikeId) {
      this.nodes = [];
      this.links = [];
      return;
    }

    this.loading = true;
    this.error = null;

    try {
      const data = await this.tauriService.getForceGraphLayout(this.bikeId);
      this.nodes = data.nodes;
      this.links = data.links;
      this.updateViewBoxFromBounds(data.bounds);
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Failed to load graph';
      console.error('Force graph error:', err);
    } finally {
      this.loading = false;
    }
  }

  /**
   * Update viewBox based on computed bounds
   *
   * Why?
   * - Backend computes actual bounding box of all nodes
   * - viewBox should encompass all nodes with padding
   * - Enables auto-fit of graph to available space
   */
  private updateViewBoxFromBounds(bounds: [number, number, number, number]): void {
    const [minX, maxX, minY, maxY] = bounds;
    const width = maxX - minX;
    const height = maxY - minY;
    this.viewBox = `${minX} ${minY} ${width} ${height}`;
  }

  private updateViewBox(): void {
    if (this.nodes.length === 0) {
      this.viewBox = `0 0 ${this.width} ${this.height}`;
    }
  }

  /**
   * Get node by ID
   */
  getNode(nodeId: string): ForceNode | undefined {
    return this.nodes.find(n => n.id === nodeId);
  }

  /**
   * Get CSS class for a node based on its type
   */
  getNodeClass(node: ForceNode): string {
    const classes = ['node', `node-${node.nodeType}`];

    // Add status-based classes
    if (node.data.type === 'issue' && node.data.resolved) {
      classes.push('node-resolved');
    }
    if (node.data.type === 'delivery') {
      classes.push(`status-${node.data.status}`);
    }

    return classes.join(' ');
  }

  /**
   * Get CSS class for a link based on its type
   */
  getLinkClass(link: ForceLink): string {
    const sourceNode = this.getNode(link.source);
    const targetNode = this.getNode(link.target);

    if (!sourceNode || !targetNode) return 'link';

    // Different styles for different connection types
    if (targetNode.nodeType === 'issue') {
      return 'link link-issue';
    }
    return 'link link-delivery';
  }

  /**
   * Get fill color for a node
   */
  getNodeColor(node: ForceNode): string {
    switch (node.nodeType) {
      case 'deliverer':
        return '#4CAF50'; // Green for deliverer
      case 'delivery':
        return this.getDeliveryColor(node);
      case 'issue':
        return this.getIssueColor(node);
      default:
        return '#9E9E9E';
    }
  }

  private getDeliveryColor(node: ForceNode): string {
    if (node.data.type !== 'delivery') return '#2196F3';

    switch (node.data.status) {
      case 'completed':
        return '#4CAF50'; // Green
      case 'ongoing':
        return '#FF9800'; // Orange
      case 'upcoming':
        return '#2196F3'; // Blue
      default:
        return '#9E9E9E';
    }
  }

  private getIssueColor(node: ForceNode): string {
    if (node.data.type !== 'issue') return '#F44336';

    if (node.data.resolved) {
      return '#9E9E9E'; // Gray for resolved
    }

    // Color by category
    switch (node.data.category) {
      case 'late':
        return '#FF9800'; // Orange
      case 'damaged':
        return '#F44336'; // Red
      case 'wrong_order':
        return '#E91E63'; // Pink
      case 'rude':
        return '#9C27B0'; // Purple
      case 'bike_problem':
        return '#795548'; // Brown
      default:
        return '#607D8B'; // Blue-gray
    }
  }

  // ============================================
  // Drag Handling
  // ============================================

  /**
   * Handle mouse down on a node - start drag
   */
  onNodeMouseDown(event: MouseEvent, node: ForceNode): void {
    event.preventDefault();
    event.stopPropagation();

    this.draggedNode = node;
    this.dragStartX = event.clientX;
    this.dragStartY = event.clientY;

    // Add global listeners for drag
    document.addEventListener('mousemove', this.onMouseMove);
    document.addEventListener('mouseup', this.onMouseUp);
  }

  /**
   * Handle mouse move during drag
   */
  private onMouseMove = (event: MouseEvent): void => {
    if (!this.draggedNode || !this.svgElement) return;

    const svg = this.svgElement.nativeElement;
    const rect = svg.getBoundingClientRect();

    // Convert screen coordinates to SVG coordinates
    const viewBoxParts = this.viewBox.split(' ').map(Number);
    const [, , vbWidth, vbHeight] = viewBoxParts;

    const scaleX = vbWidth / rect.width;
    const scaleY = vbHeight / rect.height;

    const dx = (event.clientX - this.dragStartX) * scaleX;
    const dy = (event.clientY - this.dragStartY) * scaleY;

    // Update node position locally for immediate feedback
    this.draggedNode.x += dx;
    this.draggedNode.y += dy;

    this.dragStartX = event.clientX;
    this.dragStartY = event.clientY;
  };

  /**
   * Handle mouse up - end drag and recompute layout
   */
  private onMouseUp = async (_event: MouseEvent): Promise<void> => {
    document.removeEventListener('mousemove', this.onMouseMove);
    document.removeEventListener('mouseup', this.onMouseUp);

    if (!this.draggedNode || !this.bikeId) {
      this.draggedNode = null;
      return;
    }

    const movedNode = this.draggedNode;
    this.draggedNode = null;

    // Send updated position to backend for recomputation
    try {
      const data = await this.tauriService.updateNodePosition(
        this.bikeId,
        movedNode.id,
        movedNode.x,
        movedNode.y
      );

      // Update with recomputed positions
      this.nodes = data.nodes;
      this.links = data.links;
      this.updateViewBoxFromBounds(data.bounds);
    } catch (err) {
      console.error('Failed to update node position:', err);
      // Reload original layout on error
      this.loadGraphData();
    }
  };

  // ============================================
  // Click Handling
  // ============================================

  /**
   * Handle node click
   */
  onNodeClick(event: MouseEvent, node: ForceNode): void {
    event.stopPropagation();
    this.nodeClick.emit(node);
  }

  /**
   * Handle node double-click
   */
  onNodeDblClick(event: MouseEvent, node: ForceNode): void {
    event.stopPropagation();
    this.nodeDblClick.emit(node);
  }

  // ============================================
  // Tooltip
  // ============================================

  /**
   * Get tooltip text for a node
   */
  getTooltip(node: ForceNode): string {
    switch (node.data.type) {
      case 'deliverer':
        return `${node.label} (${node.data.status})`;
      case 'delivery':
        return `${node.data.customer} - ${node.data.status}${
          node.data.rating ? ` (★${node.data.rating})` : ''
        }`;
      case 'issue':
        return `${node.data.category} issue - ${
          node.data.resolved ? 'resolved' : 'open'
        }`;
      default:
        return node.label;
    }
  }
}
