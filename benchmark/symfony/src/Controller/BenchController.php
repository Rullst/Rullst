<?php

namespace App\Controller;

use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\Routing\Annotation\Route;
use Doctrine\ORM\EntityManagerInterface;
use App\Entity\World;

class BenchController extends AbstractController
{
    #[Route('/text', name: 'text')]
    public function text(): Response
    {
        return new Response('Hello World');
    }

    #[Route('/json', name: 'json')]
    public function jsonResponse(): JsonResponse
    {
        return new JsonResponse(['message' => 'Hello World']);
    }

    #[Route('/db-single', name: 'db_single')]
    public function dbSingle(EntityManagerInterface $entityManager): JsonResponse
    {
        $world = $entityManager->getRepository(World::class)->find(1);
        return new JsonResponse(['id' => $world->getId(), 'text' => $world->getText()]);
    }

    #[Route('/html', name: 'html')]
    public function htmlRender(): Response
    {
        return $this->render('bench/index.html.twig', [
            'message' => 'Hello World',
        ]);
    }
}
