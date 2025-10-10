# ReqMan Docker Setup

This document provides comprehensive instructions for running ReqMan using Docker containers. The Docker setup includes the ReqMan application, PostgreSQL database, and Adminer for database administration.

## 🐳 Overview

The Docker setup consists of three main services:

- **reqman**: The ReqMan web application (Rust/Rocket)
- **db**: PostgreSQL database with example data
- **adminer**: Web-based database administration tool

## 📋 Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+
- At least 2GB RAM available
- At least 1GB of available disk space
- Ports 8000, 5432, and 8080 available on your system

## 🚀 Quick Start

```bash
# Start all services
docker-compose up -d

# Wait for services to be ready (30-45 seconds)
sleep 45

# Check status
docker-compose ps
```

## 🌐 Access Points

Once running, you can access:

- **ReqMan Application**: http://localhost:8000/
- **Database Admin (Adminer)**: http://localhost:8080/
- **PostgreSQL Database**: localhost:5432

## 👥 Default Users

All users have the password: `password`

| Username | Name | Email | Role |
|----------|------|-------|------|
| `alice` | Alice Johnson | alice@reqman.com | Admin |
| `dr_smith` | Dr. Sarah Smith | sarah.smith@spacecorp.com | Admin |
| `admin` | System Administrator | admin@reqman.com | Admin |
| `eng_jones` | Engineer Mike Jones | mike.jones@spacecorp.com | User |
| `tech_lee` | Technician Lisa Lee | lisa.lee@spacecorp.com | User |
| `qa_wilson` | QA Specialist Tom Wilson | tom.wilson@spacecorp.com | User |

## 📊 Example Data

The database comes pre-loaded with realistic example data:

### Projects
- **Space Project**: Complete satellite requirements management system
- **ReqMan Project**: Requirements management system development
- **Empty Project**: For testing and demonstration

### Requirements (Space Project)
- **REQ-PWR-001**: Solar array power generation (500W minimum)
- **REQ-PWR-002**: Battery system endurance (200W for 45 minutes)
- **REQ-COMM-001**: Communication coverage (90% of orbit)
- **REQ-ACS-001**: Pointing accuracy (±0.1 degrees)
- **REQ-THERM-001**: Temperature range (-20°C to +60°C)

### Tests
- **TEST-PWR-001**: Solar Array Power Output Test
- **TEST-PWR-002**: Battery Endurance Discharge Test
- **TEST-COMM-001**: S-Band Communication Performance Test
- **TEST-ACS-001**: Star Tracker Pointing Accuracy Test
- **TEST-THERM-001**: Thermal Vacuum Performance Test

### Additional Data
- **Categories**: Power System, Communication, Attitude Control, Thermal Control, Payload, Propulsion, Structure, Software
- **Applicability**: All Missions, Earth Observation, Communication, Navigation, Deep Space, CubeSat
- **Traceability Matrix**: Requirements linked to their corresponding tests

## 🛠️ Management Commands

```bash
# Start services
docker-compose up -d

# Stop services
docker-compose down

# Stop and remove volumes (deletes all data)
docker-compose down -v

# View logs
docker-compose logs -f reqman

# Check status
docker-compose ps

# Rebuild images
docker-compose build

# Rebuild and start
docker-compose up -d --build

# Reset with fresh data (deletes all data)
docker-compose down -v
docker-compose up -d
```

## 🗄️ Database Administration

### Using Adminer

1. Open http://localhost:8080/
2. Use these connection details:
   - **Server**: `db`
   - **Username**: `rust`
   - **Password**: `rust`
   - **Database**: `reqman`

### Using Command Line

```bash
# Connect to database
docker-compose exec db psql -U rust -d reqman

# Run SQL queries
docker-compose exec db psql -U rust -d reqman -c "SELECT * FROM users;"

# Export database
docker-compose exec db pg_dump -U rust reqman > backup.sql

# Import database
docker-compose exec -T db psql -U rust -d reqman < backup.sql
```

### Access Application Container

```bash
docker-compose exec reqman bash
```

## 🔧 Configuration

### Environment Variables

The application uses these environment variables:

```yaml
# Database connection
DATABASE_URL=postgres://rust:rust@db:5432/reqman
ROCKET_DATABASES={my_db={url="postgres://rust:rust@db:5432/reqman",pool_size=10}}

# Security
ROCKET_SECRET_KEY=8XuiJDYi9m8KbwspBnjctshHtRnhMxTqc6QQGJ7asJE=
```

### Database Configuration

```yaml
# PostgreSQL settings
POSTGRES_DB=reqman
POSTGRES_USER=rust
POSTGRES_PASSWORD=rust
```

## 🏗️ Architecture

### Container Details

#### ReqMan Application (`reqman`)
- **Base Image**: `rust:1.90-bookworm` (builder) → `debian:bookworm` (runtime)
- **User**: `reqman` (non-root, UID 999)
- **Port**: 8000
- **Health Check**: HTTP GET to `/`
- **Dependencies**: Database must be healthy before starting

#### PostgreSQL Database (`db`)
- **Base Image**: `postgres:15-alpine`
- **Port**: 5432
- **Health Check**: `pg_isready`
- **Data Volume**: `reqman_pgdata`
- **Initialization**: `init_complete.sql` (creates schema and sample data)

#### Adminer (`adminer`)
- **Base Image**: `adminer:4.8.1`
- **Port**: 8080
- **Purpose**: Web-based database administration

### Network

All containers run on the `reqman_reqman-network` bridge network, allowing them to communicate using service names as hostnames.

### Data Persistence

- Database data is persisted in a Docker volume named `pgdata`
- Application templates and migrations are mounted as read-only volumes
- Logs are stored in the application container

### Health Checks

- **Database:** Uses `pg_isready` to check PostgreSQL availability
- **Application:** Uses HTTP endpoint check on port 8000
- **Startup time:** Application has a 40-second startup period

## 🐛 Troubleshooting

### Common Issues

#### Application Won't Start
```bash
# Check logs
docker-compose logs reqman

# Check if database is healthy
docker-compose ps

# Restart with fresh build
docker-compose down
docker-compose up -d --build
```

#### Database Connection Issues
```bash
# Check database logs
docker-compose logs db

# Test database connectivity
docker-compose exec db pg_isready -U rust -d reqman

# Reset database
docker-compose down -v
docker-compose up -d
```

#### Port Conflicts
If ports 8000, 5432, or 8080 are already in use:

```bash
# Check what's using the ports
sudo netstat -tulpn | grep -E ':(8000|5432|8080)'

# Stop conflicting services or modify docker-compose.yml
```

#### Out of Memory
Increase Docker's memory limit or close other applications.

#### Permission Issues
```bash
# Fix file permissions
sudo chown -R $USER:$USER .

# Rebuild with no cache
docker-compose build --no-cache
```

### Health Checks

```bash
# Check all services
docker-compose ps

# Check specific service health
docker inspect reqman_reqman_1 | grep -A 10 "Health"

# Manual health check
curl -f http://localhost:8000/ || echo "Application not responding"
```

### Logs and Debugging

```bash
# View all logs
docker-compose logs

# Follow logs in real-time
docker-compose logs -f

# View specific service logs
docker-compose logs reqman
docker-compose logs db
docker-compose logs adminer

# View last 100 lines
docker-compose logs --tail=100 reqman
```

### Clean Restart
If you encounter persistent issues:
```bash
docker-compose down -v
docker-compose build
docker-compose up -d
```

## 🚀 Development Mode

For local development with hot reloading:

1. Start only the database:
   ```bash
   docker-compose up -d db adminer
   ```

2. Run the application locally:
   ```bash
   cargo run
   ```

This allows you to use your local development environment while using the containerized database.

## 📁 File Structure

```
ReqMan/
├── docker-compose.yml          # Docker services configuration
├── Dockerfile                  # ReqMan application container
├── Rocket.docker.toml         # Rocket configuration for Docker
├── .dockerignore              # Files to ignore in Docker build
├── init_complete.sql          # Database initialization script
└── populate_*.sql             # Additional data scripts
```

## 🔒 Security Considerations

- Application runs as non-root user (`reqman`)
- Database uses dedicated user (`rust`) with limited privileges
- Default passwords should be changed in production
- Secret key should be regenerated for production use
- Consider using Docker secrets for sensitive data

## 🚀 Production Deployment

For production deployment, consider:

1. **Change default passwords**:
   ```bash
   # Generate new secret key
   openssl rand -base64 32
   
   # Update docker-compose.yml with new values
   ```

2. **Use environment files**:
   ```bash
   # Create .env file
   echo "POSTGRES_PASSWORD=your_secure_password" > .env
   echo "ROCKET_SECRET_KEY=your_generated_key" >> .env
   ```

3. **Enable SSL/TLS** with reverse proxy (nginx/traefik)

4. **Set up monitoring** and logging

5. **Regular backups**:
   ```bash
   # Automated backup script
   docker-compose exec db pg_dump -U rust reqman | gzip > backup_$(date +%Y%m%d_%H%M%S).sql.gz
   ```

6. **Configure resource limits** and health checks

7. **Set up reverse proxy** (nginx/traefik)

8. **Use Docker secrets** for sensitive data

## 📞 Support

If you encounter issues:

1. Check the logs: `docker-compose logs reqman`
2. Verify status: `docker-compose ps`
3. Try resetting: `docker-compose down -v && docker-compose up -d`
4. Check this documentation for troubleshooting steps

## 📝 License

This Docker setup is part of the ReqMan project and follows the same GNU General Public License.