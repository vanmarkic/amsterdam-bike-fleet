export interface BikePosition {
  id: string;
  name: string;
  longitude: number;
  latitude: number;
  status: 'delivering' | 'idle' | 'returning';
  speed: number; // km/h
}

export interface PollutionZone {
  id: string;
  name: string;
  level: 'low' | 'moderate' | 'high';
  polygon: [number, number][]; // [lng, lat] pairs
}

export interface TrafficJam {
  id: string;
  name: string;
  severity: 'light' | 'moderate' | 'heavy';
  polygon: [number, number][]; // [lng, lat] pairs
}

export interface FleetData {
  bikes: BikePosition[];
  pollutionZones: PollutionZone[];
  trafficJams: TrafficJam[];
  timestamp: Date;
}

// Delivery tracking
export type DeliveryStatus = 'completed' | 'ongoing' | 'upcoming';

export interface Delivery {
  id: string;
  bikeId: string;
  status: DeliveryStatus;
  customerName: string;
  customerAddress: string;
  restaurantName: string;
  restaurantAddress: string;
  rating: number | null;        // 1-5, only for completed
  complaint: string | null;     // customer complaint text
  createdAt: Date;
  completedAt: Date | null;
}

// Issue tracking
export type IssueReporterType = 'customer' | 'deliverer' | 'restaurant';
export type IssueCategory = 'late' | 'damaged' | 'wrong_order' | 'rude' | 'bike_problem' | 'other';

export interface Issue {
  id: string;
  deliveryId: string | null;    // null = standalone issue
  bikeId: string;
  reporterType: IssueReporterType;
  category: IssueCategory;
  description: string;
  resolved: boolean;
  createdAt: Date;
}
