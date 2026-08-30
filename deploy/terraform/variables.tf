# deploy/terraform/variables.tf

variable "aws_region" {
  type        = string
  default     = "us-east-1"
  description = "AWS region for Route53 record"
}

variable "domain_name" {
  type        = string
  description = "The primary domain name hosted on AWS Route53 (e.g. radmuffin.click)"
}

variable "subdomain" {
  type        = string
  default     = "blist"
  description = "The subdomain to point to bList (e.g. blist)"
}

variable "fly_app_name" {
  type        = string
  description = "The unique Fly.io application name (e.g. blist-radmuffin)"
}

variable "fly_org" {
  type        = string
  default     = "personal"
  description = "Fly.io organization name"
}

variable "fly_region" {
  type        = string
  default     = "ord"
  description = "Fly.io target deployment region"
}
