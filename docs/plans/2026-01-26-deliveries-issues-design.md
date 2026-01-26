# Deliveries & Issues Feature Design

## Overview
Enrich the bike fleet app with delivery tracking and issue management. Add two new views with master-detail layouts, using Angular Router for navigation and HttpClient with interceptors for mock API simulation.

## Data Models

```typescript
interface Delivery {
  id: string;
  bikeId: string;
  status: 'completed' | 'ongoing' | 'upcoming';
  customerName: string;
  customerAddress: string;
  restaurantName: string;
  restaurantAddress: string;
  rating: number | null;        // 1-5, completed only
  complaint: string | null;
  createdAt: Date;
  completedAt: Date | null;
}

interface Issue {
  id: string;
  deliveryId: string | null;    // null = standalone
  bikeId: string;
  reporterType: 'customer' | 'deliverer' | 'restaurant';
  category: 'late' | 'damaged' | 'wrong_order' | 'rude' | 'bike_problem' | 'other';
  description: string;
  resolved: boolean;
  createdAt: Date;
}
```

## Architecture

### File Structure
```
src/app/
├── models/fleet.models.ts          # Add Delivery, Issue
├── services/
│   ├── delivery.service.ts         # NEW
│   └── issue.service.ts            # NEW
├── interceptors/
│   └── mock-api.interceptor.ts     # NEW
├── components/
│   ├── nav-tabs/                   # NEW
│   ├── deliveries-page/            # NEW - master-detail container
│   │   ├── delivery-list/          # NEW
│   │   └── delivery-detail/        # NEW
│   └── issues-page/                # NEW - master-detail container
│       ├── issue-list/             # NEW
│       └── issue-detail/           # NEW
├── app-routing.module.ts           # NEW
└── app.module.ts                   # Update
```

### Routes
- `/` → redirects to `/map`
- `/map` → FleetMapComponent
- `/deliveries` → DeliveriesPageComponent
- `/issues` → IssuesPageComponent

### API Endpoints (mocked)
- `GET /api/deliveries?status=&bikeId=`
- `GET /api/deliveries/:id`
- `GET /api/issues?resolved=&bikeId=&category=`
- `GET /api/issues/:id`

## UI Layout

### Navigation
Top tabs: Map | Deliveries | Issues

### Master-Detail (Deliveries/Issues)
- Left panel (30%): Filterable list
- Right panel (70%): Selected item detail

## Mock Data
- 50 deliveries across 20 bikes
- 20 issues (70% linked to deliveries)
- 200-400ms simulated latency
- 5% error rate for realism

## Implementation Tasks

1. **Models**: Add Delivery, Issue interfaces
2. **Interceptor**: Create mock API with data generation
3. **Services**: DeliveryService, IssueService with HttpClient
4. **Routing**: Add AppRoutingModule, update AppModule
5. **Nav tabs**: Create navigation component
6. **Deliveries page**: List + detail components
7. **Issues page**: List + detail components
8. **Refactor**: Update FleetApiService to use HttpClient
