# deploy/terraform/main.tf
terraform {
  required_version = ">= 1.0.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    fly = {
      source  = "fly-apps/fly"
      version = "~> 0.1"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

provider "fly" {
  # Expects FLY_API_TOKEN environment variable
}

# 1. AWS Route53 Records
data "aws_route53_zone" "primary" {
  name         = var.domain_name
  private_zone = false
}

# CNAME record pointing to Fly.io app
resource "aws_route53_record" "blist_cname" {
  zone_id = data.aws_route53_zone.primary.zone_id
  name    = "${var.subdomain}.${var.domain_name}"
  type    = "CNAME"
  ttl     = 300
  records = ["${var.fly_app_name}.fly.dev"]
}

# 2. Fly.io Infrastructure
resource "fly_app" "blist" {
  name = var.fly_app_name
  org  = var.fly_org
}

resource "fly_volume" "blist_data" {
  app        = fly_app.blist.name
  name       = "blist_data"
  size       = 1
  region     = var.fly_region
}
