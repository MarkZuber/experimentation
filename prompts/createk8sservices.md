Create a project in a subdirectory of the root of this repo, named backend
All of this project will be on a private machine, not exposed to the internet, and only deployed for personal use.
It should be deployed on kubernetes.  The ubuntu instance to be tested on should have k8s installed.  If it is not installed or configured properly, create scripts to configure it.
Create any k8s configurations needed to standup the service(s)
Any services we create should by default be written in rust
Any cross-communication between any of those services should use gRPC
In addition to the raw services (e.g. postgresql, redis) being installed on the pod, create any necessary service code that lives on those pods to expose the functionality.  All backend code should be in Rust

Create a pod running Postgresql.  Create a database within it, with one table named Items
Items table has these columns
- ItemId (64 bit unsigned int)
- Name (string)
- Description (string)
- DateCreated (datetime in UTC)

Use an appropriate logging service for tracking telemetry and have that live in a pod

Create another pod with Redis

Create another pod for product itself.  Call it ExperiProduct
The backend of the site should be in Rust using Actix for the web API layer
The front end of the site should be in Typescript with React
the site should have a top-bar nav and contain a welcome page with the option to sign in (via google signin). sign in attempts should be logged to the telemetry service
when signed in, the site should be shown the list of contents from the Items table
There should be the ability to sign out
The user should be able to view/add/edit/delete/sort items on the Items table
Any add/edit/delete operations on Items table should be logged to the telemetry service
The Rust service code should periodically (every few seconds) create a key/value pair that is sent to the Redis service.  These key/value pairs should have a TTL of 3 minutes.
There should be another page on the site that will show the current key/value pairs in the Redis store.  It should either refresh/poll periodically or have a way to subscribe to notifications in order to refresh.

The website should be accessible externally (from off the machine) on port 80
The website can be http since I don't have SSL certificates for my private dev machine.  If we can fake it, then use https

There should be another page, which can be navigated to via the website top-bar, that will show the telemetry activities
There should be another page that shows the architectural layout of the entire product.  each k8s pod, what's running on it, etc.  For example, a PlantUML visualization.

There must be a way to run a script that does a build of everything and deploys it all.  This script will be run from the Ubuntu Server machine that hosts the k8s instance

The script to create the pods/instances should be idempotent (and updatable) so that it can be run repeatedly to create a fresh instance or update/upgrade an existing instance.

There should be a script to uninstall/remove the services/pods from the machine so that a clean install can be performed later.

Update CLAUDE.md with any steps needed to build/test/deploy this.
Create a rules file for future use with additional requirements to use in the future such as language choice, coding style, and others you deem appropriate